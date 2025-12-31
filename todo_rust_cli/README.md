# ✅ 使い方の詳細まとめ

## ディレクトリ構成

デフォルトは `~/todo/`。

- `active/`：作業中（todo/doing/waiting）
    
- `done/YYYY/MM/`：完了（done）
    
- `canceled/YYYY/MM/`：中止（canceled）
    
- `done/unknown/` / `canceled/unknown/`：日付が取れない完了/中止
    
- `done/broken/` / `canceled/broken/`：壊れたMarkdown（frontmatter不正など）
    
- `templates/`：テンプレ置き場
    

---

## 1ファイル=1TODO、ファイル名

- 新規作成は `YYYYMMDDhhmmss__slug.md`（slugはタイトル由来）
    
- `reopen` すると **必ず** `active/` に戻り、**新しいTS+slugでリネーム**される
    

---

## frontmatter（YAML）

最終的に使っているのはこの構造（互換でOK）：

```yaml
---
id: "2025-12-31T01:23:45+09:00"
title: "..."
status: todo|doing|waiting|done|canceled
due: "2026-01-10"  # or RFC3339
tags: ["work","mail"]
importance: 3
created_at: "..."
updated_at: "..."
done_at: "..."              # done/canceledのとき
restored_from: "/path/..."  # archive復旧やreopenなど移動時
---
```

---

## fzf の挙動（超重要）

### fzfが起動する場面

- `todo done` / `todo start` / `todo wait` / `todo cancel` / `todo reopen` を **引数なし**で実行
    
- prefix指定で候補が複数ある（例：`todo edit 2025-12-31T01`）
    

### fzf中の操作

- Enter：選択確定
    
- **Ctrl-O**：選択中ファイルを `$EDITOR`（なければ nvim）で開く
    
- 右側プレビュー：
    
    - `bat`/`batcat` があれば色付き＋行番号
        
    - 無ければ `sed -n 1,200p` で簡易表示
        

---

## 基本コマンド

### 新規作成

```bash
todo add "買い物" --due 2026-01-05 --tags home,errand --importance 2
todo add "設計レビュー" --edit
```

### 一覧（締切・重要度で実用的にフィルタ）

```bash
todo list
todo list --due-within 14d
todo list --due-within 14d --include-overdue
todo list --tag work
todo list --importance ">=4"
todo list --text "k8s"
```

### 編集・表示（prefix指定可）

```bash
todo show 2025-12-31T01
todo edit 2025-12-31T01
```

### 状態変更（引数なしで即fzf）

```bash
todo start           # doingへ（activeからfzf）
todo wait            # waitingへ（activeからfzf）
todo done            # doneへ（activeからfzf）
todo cancel          # canceledへ（activeからfzf）
todo reopen          # done/canceledからfzf（archive含む）→ activeへ戻す＋リネーム
```

prefix指定もOK：

```bash
todo done 2025-12-31T01
todo reopen 2025-12-20T09
```

※ `reopen <prefix>` は **done/canceled以外を拒否**。

---

## archive（整理が強い）

```bash
todo archive
```

これ1回でやること：

1. `active/` の `done/canceled` を `done|canceled/YYYY/MM/` に移動
    
2. `done/` `canceled/` の中身を総点検し、
    
    - statusがズレてたら正しいrootへ
        
    - done_atが無ければ updated_at→created_at を代用して YYYY/MM
        
    - 日付取れない → unknown/
        
    - 壊れてる → broken/
        
    - `status: todo/doing/waiting` が紛れてたら **activeへ復旧（ログ追記＋restored_from）**
        

---

## broken 修復

```bash
todo fix-broken
```

- `done/broken/` と `canceled/broken/` を fzf で選択（プレビュー付き）
    
- `$EDITOR` で直す
    
- 直ったら自動で「statusに応じて正しい場所」へ配置
    
    - active status → active（ログ+restored_from）
        
    - done/canceled → YYYY/MM or unknown（restored_from）
        

---

# 補足：おすすめ設定（任意）

`~/.config/todo/config.toml` を作ると便利：

```toml
root_dir = "/home/you/todo"
soon_days = 7
editor = "nvim"
archive = true
auto_archive = false
```

