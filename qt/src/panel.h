// Panel positioning and focus behavior, 1:1 port of rust/src-tauri/src/panel.rs
// (SPEC sections 10, 14, 15). The slide/fade animation runs in PanelWindow
// (SPEC 15); this controller owns position, visibility, the focus-loss
// auto-hide, and the 300ms tray-click re-entry guard. All coordinates are
// logical pixels: Qt 6 is PerMonitorV2 by default, so the tao
// PhysicalPosition/LogicalSize trap does not apply (QT-MIGRATION 4.5).

#ifndef PANEL_H
#define PANEL_H

class App;

namespace panel {

// ShowNearTray: bottom-right of the primary work area, 12px margin (SPEC 10.1).
void showPanel(App *app);

// Ask the panel to play the slide-out animation (re-entry guarded).
void startHide(App *app);

// Called by the panel after the fade-out finished: actually hide + stamp
// lastHide for the 300ms tray-click guard (SPEC 14 / 15.2). A stale
// finishHide after a newer showPanel must not hide the fresh window.
void finishHide(App *app);

// TogglePopup: visible -> hide, hidden -> show. Tray left-clicks within 300ms
// of an auto-hide are ignored (same click caused the focus-loss hide first).
void togglePanel(App *app, bool fromTray);

// Deactivated auto-hide, suppressed while the settings or skills window is
// open (SPEC 20 / 21).
void onMainBlur(App *app);

} // namespace panel

#endif // PANEL_H
