using System.Diagnostics;
using System.Net.Http;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Windows;

namespace KimiPlanbarTray;

// 新版本检测：本地 kimi --version vs GitHub Releases latest
public class UpdateService
{
    private static readonly HttpClient Http = new() { Timeout = TimeSpan.FromSeconds(10) };
    public string? LocalVersion { get; private set; }
    public string? LatestVersion { get; private set; }
    public bool UpdateAvailable { get; private set; }
    public bool CheckFailed { get; private set; }
    public event Action? Updated;

    public async Task CheckAsync()
    {
        // 子进程调用放线程池，避免阻塞 UI 线程
        LocalVersion = await Task.Run(DetectLocalVersion).ConfigureAwait(false);
        try
        {
            using var req = new HttpRequestMessage(HttpMethod.Get,
                "https://api.github.com/repos/MoonshotAI/kimi-code/releases/latest");
            req.Headers.TryAddWithoutValidation("User-Agent", "KimiPlanbarTray");
            using var resp = await Http.SendAsync(req).ConfigureAwait(false);
            resp.EnsureSuccessStatusCode();
            using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync().ConfigureAwait(false));
            var tag = doc.RootElement.GetProperty("tag_name").GetString();
            // tag 形如 "@moonshot-ai/kimi-code@0.31.1"，直接提取版本号
            var m = Regex.Match(tag ?? "", @"\d+\.\d+\.\d+");
            LatestVersion = m.Success ? m.Value : tag;
            UpdateAvailable = m.Success
                              && Version.TryParse(m.Value, out var latest)
                              && Version.TryParse(LocalVersion, out var local)
                              && latest > local;
            CheckFailed = false;
        }
        catch
        {
            // 网络不可达（如本机 hosts 屏蔽 api.github.com）时静默降级
            CheckFailed = true;
        }
        try { Application.Current?.Dispatcher.BeginInvoke(() => Updated?.Invoke()); } catch { }
    }

    private static string? DetectLocalVersion()
    {
        Process? p = null;
        try
        {
            var psi = new ProcessStartInfo("kimi", "--version")
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            p = Process.Start(psi);
            if (p == null) return null;
            // 先限时等待再读输出：kimi --version 输出仅一行，不会撑满管道缓冲
            if (!p.WaitForExit(5000))
            {
                try { p.Kill(); } catch { }
                return null;
            }
            string outp = p.StandardOutput.ReadToEnd() + p.StandardError.ReadToEnd();
            var m = Regex.Match(outp, @"\d+\.\d+\.\d+");
            return m.Success ? m.Value : null;
        }
        catch { return null; }
        finally { p?.Dispose(); }
    }
}
