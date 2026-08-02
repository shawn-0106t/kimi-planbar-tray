using System.Net.Http;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Windows;

namespace KimiPlanbarTray;

public class QuotaSegment
{
    public double Percent { get; set; }
    public DateTimeOffset? ResetAt { get; set; }
}

public enum ExtraState { NotActivated, NoData, Ready }

// Extra Usage（boosterWallet）：余额三态 + 月度已用/上限
public class ExtraInfo
{
    public ExtraState State { get; set; }
    public long? BalanceCents { get; set; }      // amountLeft（1e-8 元）换算到分
    public bool MonthlyEnabled { get; set; }
    public long? MonthlyUsedCents { get; set; }
    public long? MonthlyLimitCents { get; set; }
}

public class QuotaResult
{
    public QuotaSegment? FiveHour { get; set; }
    public QuotaSegment? Week { get; set; }
    public ExtraInfo? Extra { get; set; }
    public DateTimeOffset FetchedAt { get; set; }
    public string? Error { get; set; }
}

// 数据逻辑移植自 quota-status.py：token 来源与解析规则保持一致
public class QuotaService : IDisposable
{
    private static readonly HttpClient Http = new() { Timeout = TimeSpan.FromSeconds(10) };
    private readonly Timer _timer;
    private int _periodMs;
    public QuotaResult? Last { get; private set; }
    public event Action? Updated;

    public QuotaService()
    {
        _timer = new Timer(async _ => await SafeRefresh(), null, Timeout.Infinite, Timeout.Infinite);
    }

    public void StartAutoRefresh() => Reschedule();

    public void Reschedule()
    {
        _periodMs = (int)TimeSpan.FromMinutes(Math.Max(1, App.Settings.Data.RefreshMinutes)).TotalMilliseconds;
        _timer.Change(TimeSpan.FromSeconds(2), TimeSpan.FromMilliseconds(_periodMs));
    }

    public async Task SafeRefresh()
    {
        var r = await FetchAsync();
        if (r != null)
        {
            // 失败时保留上一次成功的数据，界面不清空（仅状态行提示更新失败）
            if (r.Error != null && Last != null)
            {
                r.FiveHour ??= Last.FiveHour;
                r.Week ??= Last.Week;
                r.Extra ??= Last.Extra;
            }
            Last = r;
            // 失败后 30 秒快速重试（对齐 quota-status.py 的 RETRY 策略），成功则回到正常周期
            // （退出阶段 Timer 可能已 Dispose，吞掉竞态异常）
            try
            {
                _timer.Change(TimeSpan.FromMilliseconds(r.Error != null ? 30_000 : _periodMs),
                              TimeSpan.FromMilliseconds(_periodMs));
            }
            catch (ObjectDisposedException) { }
        }
        try { Application.Current?.Dispatcher.Invoke(() => Updated?.Invoke()); } catch { }
    }

    // 截图/测试模式注入数据
    internal void Inject(QuotaResult? r) => Last = r;

    public async Task<QuotaResult?> FetchAsync()
    {
        try
        {
            var token = LoadToken();
            if (token == null)
                return new QuotaResult { Error = "no-token", FetchedAt = DateTimeOffset.Now };
            using var req = new HttpRequestMessage(HttpMethod.Get, "https://api.kimi.com/coding/v1/usages");
            req.Headers.TryAddWithoutValidation("Authorization", $"Bearer {token}");
            req.Headers.TryAddWithoutValidation("Accept", "application/json");
            using var resp = await Http.SendAsync(req).ConfigureAwait(false);
            resp.EnsureSuccessStatusCode();
            using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync().ConfigureAwait(false));
            var root = doc.RootElement;
            var r = new QuotaResult { FetchedAt = DateTimeOffset.Now };
            if (root.TryGetProperty("limits", out var lims)
                && lims.ValueKind == JsonValueKind.Array && lims.GetArrayLength() > 0)
            {
                var det = lims[0].GetPropertyOrDefault("detail");
                if (det.HasValue) r.FiveHour = ParseSegment(det.Value);
            }
            if (root.TryGetProperty("usage", out var u) && u.ValueKind == JsonValueKind.Object)
                r.Week = ParseSegment(u);
            r.Extra = ParseExtra(root.GetPropertyOrDefault("boosterWallet"));
            return r;
        }
        catch (Exception ex)
        {
            return new QuotaResult { Error = ex.GetType().Name, FetchedAt = DateTimeOffset.Now };
        }
    }

    // boosterWallet：余额三态（未开通/无数据/有值）+ 月度已用/上限
    // 注意 JSON 数字皆为字符串；amountLeft 单位 1e-8 元，priceInCents 单位分
    private static ExtraInfo ParseExtra(JsonElement? wallet)
    {
        var info = new ExtraInfo();
        if (wallet == null || wallet.Value.ValueKind != JsonValueKind.Object)
        {
            info.State = ExtraState.NotActivated;
            return info;
        }
        var w = wallet.Value;

        // isEnabled 防御（借鉴 KimiCodeBar v1.1.1 的 bug）：booster 未启用时
        // 接口返回的 amountLeft 是"月度上限-已用"估算值而非真实余额，必须视为未开通
        if (w.TryGetProperty("isEnabled", out var ie)
            && ie.ValueKind == JsonValueKind.False)
        {
            info.State = ExtraState.NotActivated;
            return info;
        }

        if (w.GetPropertyOrDefault("balance") is { ValueKind: JsonValueKind.Object } bal
            && bal.GetPropertyOrDefault("amountLeft") is JsonElement al
            && TryGetLong(al, out long raw))
        {
            info.State = ExtraState.Ready;
            info.BalanceCents = (raw + 500000) / 1000000; // 1e-8 元 → 分，四舍五入
        }
        else
        {
            info.State = ExtraState.NoData;
        }

        if (w.TryGetProperty("monthlyChargeLimitEnabled", out var en)
            && en.ValueKind == JsonValueKind.True)
        {
            info.MonthlyEnabled = true;
            info.MonthlyUsedCents = ParseCents(w.GetPropertyOrDefault("monthlyUsed"));
            info.MonthlyLimitCents = ParseCents(w.GetPropertyOrDefault("monthlyChargeLimit"));
        }
        return info;
    }

    // JSON 数字按字符串建模（服务端惯例），容忍数字型兜底
    private static bool TryGetLong(JsonElement e, out long v)
    {
        if (e.ValueKind == JsonValueKind.String) return long.TryParse(e.GetString(), out v);
        if (e.ValueKind == JsonValueKind.Number) return e.TryGetInt64(out v);
        v = 0;
        return false;
    }

    private static long? ParseCents(JsonElement? money)
    {
        if (money is { ValueKind: JsonValueKind.Object } m
            && m.TryGetProperty("priceInCents", out var p)
            && TryGetLong(p, out long v))
            return v;
        return null;
    }

    private static QuotaSegment ParseSegment(JsonElement e)
    {
        double used = e.GetDoubleOrZero("used");
        double limit = e.GetDoubleOrZero("limit");
        if (limit <= 0) limit = 1;
        var seg = new QuotaSegment { Percent = used / limit * 100 };
        if (e.TryGetProperty("resetTime", out var rt) && rt.ValueKind == JsonValueKind.String
            && DateTimeOffset.TryParse(rt.GetString(), out var dto))
            seg.ResetAt = dto;
        return seg;
    }

    // 1) ~/.kimi-code/credentials/kimi-code.json 的 OAuth access_token（未过期）
    // 2) config.toml 中 base_url 含 api.kimi.com/coding 的 provider 的明文 api_key
    public static string? LoadToken()
    {
        var home = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".kimi-code");
        try
        {
            var cred = Path.Combine(home, "credentials", "kimi-code.json");
            if (File.Exists(cred))
            {
                using var doc = JsonDocument.Parse(File.ReadAllText(cred));
                var root = doc.RootElement;
                if (root.TryGetProperty("access_token", out var at) && at.ValueKind == JsonValueKind.String)
                {
                    double exp = root.GetDoubleOrZero("expires_at");
                    if (exp > DateTimeOffset.UtcNow.ToUnixTimeSeconds() + 30)
                        return at.GetString();
                }
            }
        }
        catch { }

        try
        {
            var cfgPath = Path.Combine(home, "config.toml");
            if (!File.Exists(cfgPath)) return null;
            string? section = null, baseUrl = null, apiKey = null;
            foreach (var raw in File.ReadLines(cfgPath))
            {
                var line = raw.Trim();
                if (line.StartsWith('['))
                {
                    var found = MatchProvider(section, baseUrl, apiKey);
                    if (found != null) return found;
                    section = line.Trim('[', ']');
                    baseUrl = apiKey = null;
                    continue;
                }
                var m = Regex.Match(line, "^(base_url|api_key)\\s*=\\s*\"([^\"]*)\"");
                if (!m.Success) continue;
                if (m.Groups[1].Value == "base_url") baseUrl = m.Groups[2].Value;
                else apiKey = m.Groups[2].Value;
            }
            return MatchProvider(section, baseUrl, apiKey);
        }
        catch { return null; }
    }

    private static string? MatchProvider(string? section, string? baseUrl, string? apiKey)
        => section != null && section.StartsWith("providers.")
           && baseUrl != null && baseUrl.Contains("api.kimi.com/coding")
           && !string.IsNullOrEmpty(apiKey)
            ? apiKey : null;

    public void Dispose() => _timer.Dispose();
}

internal static class JsonExt
{
    public static JsonElement? GetPropertyOrDefault(this JsonElement e, string name)
        => e.ValueKind == JsonValueKind.Object && e.TryGetProperty(name, out var v) ? v : null;

    public static double GetDoubleOrZero(this JsonElement e, string name)
    {
        if (e.ValueKind == JsonValueKind.Object && e.TryGetProperty(name, out var v))
        {
            if (v.ValueKind == JsonValueKind.Number && v.TryGetDouble(out var d)) return d;
            if (v.ValueKind == JsonValueKind.String && double.TryParse(v.GetString(), out var ds)) return ds;
        }
        return 0;
    }
}
