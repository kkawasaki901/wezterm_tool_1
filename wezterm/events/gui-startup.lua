-- events/gui-startup.lua
local wezterm = require 'wezterm'
local mux = wezterm.mux

local M = {}
local done = false

function M.setup()
  wezterm.on('gui-startup', function(cmd)
    if done then return end
    done = true

    -- ★ ここで1回だけウィンドウ生成
    -- tasksペイン
    local tab1, pane1, window = mux.spawn_window({args = {  "wsl.exe", "--exec", "bash", "-lc", "nb todo list; exec bash -l" },})
    -- window:gui_window():maximize()
    window:gui_window():maximize()

    -- Tab 1: 左3段 + 中(inbox) + 右(org) 
    local pane_org = pane1:split { direction = 'Right', size = 0.12,  args = { "wsl.exe", "--exec", "bash", "-lc", "emacs -nw" }}
    local pane_inbox = pane1:split { direction = 'Right', size = 0.85 , args = { "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/obsidian/2026_test/Inbox'; nvim ."}}
    local pane_daily = pane1:split { direction = 'Bottom', size = 0.66 , args = { "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/obsidian/2026_test/Daily'; nvim ."}}
    local pane_terminal = pane_daily:split { direction = 'Bottom', size = 0.50, args = { "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/go/bin'; ./english.exe"}}
    tab1:set_title('main')

    -- Tab 2: 3列
    local tab2, pane2 = window:spawn_tab {args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/question'; edit ."}}
    local pane_scratch = pane2:split { direction = 'Right', size = 11/12, args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/scratch'; edit ."}}
    local pane_keep = pane_scratch:split { direction = 'Right', size = 3/4 , args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/keep'; edit ."}}

    tab2:set_title('study')
    
    -- Tab 3: free
    -- local tab3, pane3 = window:spawn_tab {args = {  "pwsh.exe", "-NoLogo", "-NoExit","-Command", "Set-Location 'C:/Users/kohei/edit/other'; hx ."}}
  end)
end

return M
