# MiniVisor
[作って理解する仮想化技術─⁠─ ハイパーバイザを実装しながら仕組みを学ぶ（技術評論社,2025）](https://gihyo.jp/book/2025/978-4-297-15012-9)で実装する[MiniVisor](https://github.com/PG-MANA/MiniVisor)を実機に移植するためのリポジトリです。

書籍やMiniVisorの問題については、[MiniVisor](https://github.com/PG-MANA/MiniVisor)に報告していただけると幸いです。

## ブランチ

- `main` : [MiniVisor](https://github.com/PG-MANA/MiniVisor) の `main`と同じ
- `real` : 物理デバイスに移植するうえで必要な共通の修正
- デバイス名 : 各デバイスで動作させるために必要な修正

本ブランチは[ODROID M1S](https://www.hardkernel.com/shop/odroid-m1s-with-8gbyte-ram/)で動作させるためのブランチです。
方法については[ODROID-M1S.md](./ODROID-M1S.md)を参照してください。

## ライセンスについて
本ソフトウェアはApache License, Version 2.0にてライセンスされています。
詳しくは[NOTICE](NOTICE)をご覧ください。
