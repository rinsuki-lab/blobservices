## 開発時お役立ち情報

* マイグレーションは blobmanager で `./generate_migration.sh 名前` で生成すること (自動diffで済む場合手動編集はしてはいけない)。

## コンポーネント間の信頼関係

* blobgateway は blobmanager / blobstore のレスポンスを信頼する。
* blobstore は blobmanager のレスポンスを信頼する。
* blobmanager は unsafe 系の入力を除いて何も信頼しない。
  * (unsafe系は後々特権ユーザーでの認証を要求するようにする)

すべてのコンポーネントは (configがあれば) configファイルと起動時の環境変数を無条件に信頼し、configファイルが間違っていた場合はサーバー管理者の責とする。
