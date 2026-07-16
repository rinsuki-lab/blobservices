# blobservices

貧者のためのS3

## 構成図

```mermaid
flowchart LR
    Client_A((Client))
    Client_B((Client))

    subgraph Region A
        subgraph blobgateway_A
            blobgateway_A1[blobgateway]
        end
        subgraph blobstore_A1[blobstore]
            blobstore_local_A1[blobstore_local]
        end
        subgraph blobstore_A2[blobstore]
            blobstore_local_A2[blobstore_local]
        end
    end
    subgraph Region B
        subgraph blobgateway_B
            blobgateway_B1[blobgateway]
        end
        subgraph blobstore_B1[blobstore]
            blobstore_local_B1[blobstore_local]
        end
    end
    subgraph Region C
        blobmanager
        Postgres
    end

    Client_A --> blobgateway_A1
    Client_B --> blobgateway_B1
    blobgateway_A1 --> blobstore_local_A1
    blobgateway_A1 --> blobstore_local_A2
    blobgateway_A1 --> blobstore_local_B1
    blobgateway_B1 --> blobstore_local_A1
    blobgateway_B1 --> blobstore_local_A2
    blobgateway_B1 --> blobstore_local_B1
    blobgateway_A1 --> blobmanager
    blobgateway_B1 --> blobmanager
    blobmanager --> Postgres
```

## コンポーネント

### blobmanager

blob のメタ情報 (サイズ、チェックサム、どのstoreにどのblobがあるかなど) を保持・提供している。
blob のデータ本体には関わらない (ので帯域がそんなに太くないサーバーに置いても大丈夫)。

### blobstore_*

blob のデータ本体を保持している (逆にメタデータなどは保存していない)。
現在はローカルのファイルシステム用の実装 (`blobstore_local`) しか実装がないが、理論上は (S3など) ほとんどすべてのストレージに対応可能。

### blobgateway

blobmanager からメタ情報を、 blobstore からデータを取得し、クライアントに提供する。
クライアントは blobgateway としか通信しない。
