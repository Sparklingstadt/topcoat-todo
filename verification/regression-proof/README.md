# 画像回帰テストの差分検出確認

2026-09-19、Linux ARM64 / Playwright 1.63.0 / Chromiumで確認。

混在一覧画面の見出しに、テストブラウザー内だけで `h1 { color: #ff00ff !important; }` を適用し、既存の画像比較テストを実行しました。

- 結果: 想定どおり **1件失敗、終了コード1**
- 画像比較: **587ピクセル**の差分を検出
- 基準画像: 26枚すべてのSHA-256を前後で比較し、変更がないことを確認
- 一時的なテスト変更: 検証後に復元
- アプリのCSSと利用中のDB: この検証では変更なし

| 画像 | 内容 |
| --- | --- |
| [expected.png](expected.png) | 保存済みの期待画像 |
| [actual.png](actual.png) | 見出しを変更した実際の画像 |
| [diff.png](diff.png) | Playwrightが生成した差分画像 |

実行結果の要約は [result.json](result.json)。最終的な全テストの結果は、プロジェクトの `VERIFICATION.md` と `playwright-report/` を参照してください。
