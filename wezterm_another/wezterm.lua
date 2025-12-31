local wezterm = require("wezterm")
local mux = wezterm.mux

local config = {}

-- ====== 小さい部品（ここだけ覚えればOK） ======
local function Img(path, opt)
  opt = opt or {}
  return {
    source = { File = path },
    horizontal_align = opt.horizontal_align or "Center",
    vertical_align   = opt.vertical_align   or "Middle",
  }
end

local function Film(color, opacity, opt)
  opt = opt or {}
  return {
    source = { Color = color },
    opacity = opacity,

    -- 画像の端が見える/隙間が出るのを防ぐ定番
    width  = opt.width  or "120%",
    height = opt.height or "120%",
    horizontal_offset = opt.horizontal_offset or "-10%",
    vertical_offset   = opt.vertical_offset   or "-10%",
  }
end

-- ====== ここから設定（読みやすい） ======
local BG_IMG = "G:/マイドライブ/picture/原神/1703016642252.jpg"
local OVERLAY_COLOR = "#0b1020"
local OVERLAY_OPACITY = 0.92

config.background = {
  Img(BG_IMG),
  Film(OVERLAY_COLOR, OVERLAY_OPACITY),
}

-- Windowsならガラス化を避けるのに安全
config.window_background_opacity = 1.0
config.text_background_opacity = 1.0


config.keys = {
  {
    key = 'Z',
    mods = 'CTRL',
    action = wezterm.action.TogglePaneZoomState,
  },
}

-- ===== 初期レイアウト =====
wezterm.on("gui-startup", function(cmd)

    -- ★ ここで1回だけウィンドウ生成
    -- tasksペイン
    local tab1, pane1, window = mux.spawn_window({args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/todo'; todo list" },})
    -- window:gui_window():maximize()
    window:gui_window():maximize()

    -- Tab 1: 左3段 + 中(inbox) + 右(org) 
    local pane_org = pane1:split { direction = 'Right', size = 0.12,  args = { "wsl.exe", "--exec", "bash", "-lc", "emacs -nw" }}
    local pane_inbox = pane1:split { direction = 'Right', size = 0.85 , args = { "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/obsidian/2026_test/Inbox'; edit ."}}
    local pane_daily = pane1:split { direction = 'Bottom', size = 0.66 , args = { "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/obsidian/2026_test/Daily'; edit ."}}
    local pane_terminal = pane_daily:split { direction = 'Bottom', size = 0.50, args = { "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/go/bin'; ./english.exe"}}
    tab1:set_title('main')

    -- Tab 2: 3列
    local tab2, pane2 = window:spawn_tab {args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/question'; edit ."}}
    local pane_scratch = pane2:split { direction = 'Right', size = 11/12, args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/scratch'; edit ."}}
    local pane_keep = pane_scratch:split { direction = 'Right', size = 3/4 , args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/keep'; edit ."}}

    tab2:set_title('study')

	window:gui_window():maximize()
end)

return config
