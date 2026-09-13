# Qt edition of Kimi Planbar Tray — C++ Qt6 + Qt Widgets (no WebView dependency).
# Plan & module mapping: ../docs/archive/QT-MIGRATION.md. Version synced with rust/
# since SPEC parity was reached (Phase 4).
cmake_minimum_required(VERSION 3.21)

project(kimi-planbar-tray VERSION 1.7.2 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets Network Svg)

qt_standard_project_setup()

qt_add_executable(kimi-planbar-tray
    src/main.cpp
    src/credentials.cpp
    src/credentials.h
    src/quota.cpp
    src/quota.h
    src/settings.cpp
    src/settings.h
    src/update.cpp
    src/update.h
    src/state.cpp
    src/state.h
    src/theme.cpp
    src/theme.h
    src/panel.cpp
    src/panel.h
    src/trayicon.cpp
    src/trayicon.h
    src/polling.cpp
    src/polling.h
    src/app.cpp
    src/app.h
    src/http_util.h
    src/async_util.h
    src/format_util.h
    src/icons.h
    src/windows/panelwindow.cpp
    src/windows/panelwindow.h
    src/windows/settingswindow.cpp
    src/windows/settingswindow.h
    src/windows/skillswindow.cpp
    src/windows/skillswindow.h
    src/windows/menuwindow.cpp
    src/windows/menuwindow.h
    src/windows/titlebar.h
    src/windows/clamplabel.h
    src/windows/hoverglow.h
    src/skills.cpp
    src/skills.h
    resources.qrc
    resources.rc
)

target_link_libraries(kimi-planbar-tray PRIVATE Qt6::Widgets Qt6::Network Qt6::Svg dwmapi)

# Sources are UTF-8; MSVC on a GBK-locale host needs this to parse them right
if(MSVC)
    target_compile_options(kimi-planbar-tray PRIVATE /utf-8)
endif()

# GUI subsystem (no console window); --test-* self-checks attach to the parent
# console at runtime via AttachConsole (QT-MIGRATION.md 4.6).
set_target_properties(kimi-planbar-tray PROPERTIES
    WIN32_EXECUTABLE TRUE
)
