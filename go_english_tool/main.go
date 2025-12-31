package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
	"time"
	"unicode/utf8"
)

type Config struct {
	WordFile string `json:"word_file"`
	Count    int    `json:"count"`
}

type Pair struct {
	EN string
	JA string
}

func main() {
	cfg, err := loadConfig()
	if err != nil {
		exitf("config error: %v\n", err)
	}

	pairs, err := loadWordPairs(cfg.WordFile)
	if err != nil {
		exitf("word file error: %v\n", err)
	}
	if len(pairs) == 0 {
		exitf("no word pairs found.\n")
	}

	count := cfg.Count
	if count <= 0 {
		count = 1
	}
	if count > len(pairs) {
		count = len(pairs)
	}

	// ランダムシャッフル
	rand.New(rand.NewSource(time.Now().UnixNano())).Shuffle(len(pairs), func(i, j int) {
		pairs[i], pairs[j] = pairs[j], pairs[i]
	})

	printAligned(pairs[:count])
}

func loadConfig() (Config, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return Config{}, err
	}

	configPath := filepath.Join(home, "english", "config.json")

	b, err := os.ReadFile(configPath)
	if err != nil {
		return Config{}, fmt.Errorf("cannot read %s: %w", configPath, err)
	}

	var cfg Config
	if err := json.Unmarshal(b, &cfg); err != nil {
		return Config{}, fmt.Errorf("parse %s: %w", configPath, err)
	}

	if strings.TrimSpace(cfg.WordFile) == "" {
		return Config{}, fmt.Errorf("word_file is empty in %s", configPath)
	}

	// 相対パスなら ~/english 基準で解決
	if !filepath.IsAbs(cfg.WordFile) {
		cfg.WordFile = filepath.Join(filepath.Dir(configPath), cfg.WordFile)
	}

	return cfg, nil
}

func loadWordPairs(path string) ([]Pair, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var pairs []Pair
	sc := bufio.NewScanner(f)

	inHeader := true // 先頭から最初の空行までは補足情報

	for sc.Scan() {
		line := strings.TrimRight(sc.Text(), "\r\n")
		trim := strings.TrimSpace(line)

		if inHeader {
			if trim == "" {
				inHeader = false
			}
			continue
		}

		if trim == "" {
			continue
		}
		if strings.HasPrefix(trim, "#") || strings.HasPrefix(trim, "//") {
			continue
		}

		en, ja, ok := splitPair(trim)
		if !ok {
			continue
		}

		pairs = append(pairs, Pair{EN: en, JA: ja})
	}

	if err := sc.Err(); err != nil {
		return nil, err
	}

	return pairs, nil
}

// "apple:リンゴ" / "apple：リンゴ" 両対応
func splitPair(s string) (string, string, bool) {
	s = strings.ReplaceAll(s, "：", ":")

	parts := strings.SplitN(s, ":", 2)
	if len(parts) != 2 {
		return "", "", false
	}

	en := strings.TrimSpace(parts[0])
	ja := strings.TrimSpace(parts[1])

	if en == "" || ja == "" {
		return "", "", false
	}

	return en, ja, true
}

func printAligned(pairs []Pair) {
	maxEN := 0
	for _, p := range pairs {
		w := utf8.RuneCountInString(p.EN)
		if w > maxEN {
			maxEN = w
		}
	}

	const gap = 4

	for _, p := range pairs {
		enWidth := utf8.RuneCountInString(p.EN)
		pad := (maxEN - enWidth) + gap
		if pad < gap {
			pad = gap
		}

		fmt.Printf("%s%s%s\n", p.EN, strings.Repeat(" ", pad), p.JA)
	}
}

func exitf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format, args...)
	os.Exit(1)
}
