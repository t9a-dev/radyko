station_idとstart_atで一意になる

# urlの例
https://tf-f-rpaa-radiko.smartstream.ne.jp/tf/playlist.m3u8?station_id=LFR&start_at=20260426010000&ft=20260426010000&end_at=20260426030000&to=20260426030000&preroll=2&l=15&lsid=4e7732feaa6e0313659cdc367bdb9036&type=b

https://tf-f-rpaa-radiko.smartstream.ne.jp
/tf/playlist.m3u8
station_id=LFR
start_at=20260426010000
ft=20260426010000
end_at=20260426030000
to=20260426030000
preroll=2
l=15
lsid=4e7732feaa6e0313659cdc367bdb9036
type=b

## タイムフリーのエンドポイントはseek可能
タイムフリーではseekパラメーターで指定した時点のmedialistが返される。
medialistには5秒のセグメントが3つ並んでいる。(lパラメーターの15はこれを指し示している？)
preroll=0にもなっている。
start_at~end_atを15秒単位で分割した`%Y%m%d%H%M%S`を計算してseekパラメーターに指定すれば全てのセグメントを取得できる
セグメント自体はmedialistの`#EXT-X-MEDIA-SEQUENCE`とセグメントのインデックスをファイル名にすれば単一ファイルへの結合時の並び順として使える。

https://tf-f-rpaa-radiko.smartstream.ne.jp/tf/playlist.m3u8?station_id=LFR&start_at=20260425010000&ft=20260425010000&end_at=20260425030000&to=20260425030000&seek=20260425022823&preroll=0&l=15&lsid=318c1ca1e74a6d9ca55ee8eff65df857&type=b

host
	tf-f-rpaa-radiko.smartstream.ne.jp
filename
	/tf/playlist.m3u8
station_id
	LFR
start_at
	20260425010000
ft
	20260425010000
end_at
	20260425030000
to
	20260425030000
seek
	20260425022823
preroll
	0
l
	15
lsid
	318c1ca1e74a6d9ca55ee8eff65df857
type
	b