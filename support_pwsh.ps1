

function fe {
  param(
    [string]$Dir = "."
  )

  # mdファイル一覧
  $items = fd -e md $Dir | ForEach-Object {
    $path = $_

    # frontmatter中の due: を拾う（無ければ $null）
    $due = $null
    try {
      $lines = Get-Content -LiteralPath $path -TotalCount 80 -ErrorAction Stop
      if ($lines.Count -gt 0 -and $lines[0] -eq "---") {
        for ($i=1; $i -lt $lines.Count; $i++) {
          if ($lines[$i] -eq "---") { break }
          if ($lines[$i] -match '^\s*due\s*:\s*(.+?)\s*$') {
            $raw = $Matches[1].Trim().Trim("'`"")
            # 例: 2025-12-31 / 2025-12-31T10:00 / 2025/12/31 などを受ける
            try { $due = [datetime]::Parse($raw) } catch { $due = $null }
            break
          }
        }
      }
    } catch {}

    [pscustomobject]@{
      Path = $path
      Due  = $due
      Key  = if ($due) { $due } else { [datetime]::MinValue }  # due無しは最後へ
    }
  }

  # due の未来が上（降順）になるように並べ替え
  $sorted = $items | Sort-Object -Property Key 
  # fzfに渡す（表示は「due\tpath」にして見やすく）
  $selected = $sorted | ForEach-Object {
    $dueStr = if ($_.Due) { $_.Due.ToString("yyyy-MM-dd") } else { "---- -- --" }
    "$dueStr`t$($_.Path)"
  } | fzf --multi --with-nth=1,2 --delimiter "`t" `
          --preview 'bat --style=numbers --color=always --line-range :200 {2}'

  if ($selected) {
    # 選択結果からパス（2列目）だけ取り出して nvim
    $paths = $selected | ForEach-Object { ($_ -split "`t", 2)[1] }
    nvim @($paths)
  }
}




function todoe {
  todo add $args | % { if ($_ -match '^created:\s+(.*)$') { nvim $Matches[1] } }
}

