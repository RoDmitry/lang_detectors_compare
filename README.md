# Natural language detectors comparison

Look inside the [`accuracy` folder](https://github.com/RoDmitry/lang_detectors_compare/tree/main/accuracy).

### Speed

| Language count | Alphabet detector | Langram max trigrams  | Langram all ngrams | Lingua high | Whatlang | Whichlang |
| --- | ------ | ------- | ------- | ------- | ------ | ------ |
|  16 |        |  0.39 s |  0.60 s |         |        | 0.02 s |
|  69 |        |  2.72 s |  5.21 s |         | 6.71 s |
|  74 |        |  4.50 s |  8.87 s | 43.57 s |
| 201 | 1.67 s | 28.64 s | 53.33 s |

> Note: 69 are different languages from 74, so they are fast detected by `alphabet_detector`.

### Texts source

Uses [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`, `zho` to `cmn`, `est` to `ekk`, `tgl` to `fil`, `grn` to `gug`, `kon` to `ktu`.

`for file in *; do head -n 2000 "$file" > "../lang_detectors_compare/texts/${file}"; done`
