Общий принцип проверки

После любого этапа или фикса проверяем 3 слоя:

архитектура

функциональность

стабильность

Stage 1 — Relay-first dataplane core
Архитектура

 relay остаётся blind forwarder

 exit делает реальный TCP connect к target

 client ↔ exit работают как end-to-end dataplane

 transport layer не захардкожен архитектурно только под будущее

 route memory модуль не удалён

Функциональность

 client -> relay -> exit -> target проходит

 curl http://127.0.0.1:10080/ даёт HTTP 200

 target_unavailable_returns_error даёт ожидаемую ошибку

 multi_frame_single_stream_request работает

 multiple_parallel_streams работает

Стабильность

 cargo build проходит

 cargo test --lib проходит

 docker / local run path задокументирован

 tcp mode не зависает

 клиент не падает после одного запроса

Stage 2 — TUN datapath
Архитектура

 tcp mode не сломан

 tun mode отделён от tcp mode

 TUN mode не доступен на non-Unix и даёт явную ошибку

 packet contract для TUN описан

 flow table двунаправленный

Функциональность

 TUN forward path работает

 TUN reverse path работает

 FlowKey <-> stream_id mapping корректен

 cleanup по Error / CloseStream есть

 idle cleanup есть

Стабильность

 cargo build проходит

 cargo test --lib проходит

 логические TUN tests проходят

 README честно описывает ограничения TUN mode

 Windows tcp mode остаётся рабочим и не притворяется TUN

Stage 3 — Runtime handshake + session crypto
Архитектура

 relay не участвует в crypto

 handshake только client ↔ exit

 один end-to-end SessionCrypto

 dev PSK больше не основной runtime path

Функциональность

 runtime handshake реально происходит

 x25519 реально используется

 HKDF реально выводит session key

 после handshake DATA идёт через session-derived AEAD

 данные не принимаются до established session

Стабильность

 replay protection включена

 duplicate packets отклоняются

 wrong / stale sequence отклоняется

 handshake timeout обрабатывается

 docker e2e после Stage 3 по-прежнему даёт HTTP 200

Stage 3.1 — Handshake hardening
Архитектура

 exit не создаёт session state до проверки cookie

 x25519 не вызывается до verify cookie

 relay остаётся blind forwarder

 handshake flow: init -> challenge -> init+cookie -> ack

Функциональность

 HandshakeChallenge есть в протоколе

 stateless cookie работает

 cookie привязан к IP + pubkey + nonce + ttl

 invalid cookie отклоняется

 rate limiting на exit работает

Стабильность

 cargo test --lib проходит

 cookie tests проходят

 docker e2e по-прежнему даёт HTTP 200

 README описывает Stage 3.1 hardening

 нет regressions в Stage 1/2/3

Stage 4 — Reliable streaming
Архитектура

 reliability реализована per stream, а не глобально на session

 нет session-wide head-of-line blocking

 relay не хранит stream state

 не построен “второй TCP стек”

 нет per-hop retransmit/session logic

Функциональность

 sequence numbering работает

 ACK работают

 retransmit работает

 out-of-order buffering работает

 end-of-stream корректно доходит

 large response доходит без обрезания

Стабильность

 есть лимит max_inflight_frames

 нет packet storm при retransmit

 нет premature CloseStream

 large_http_transfer проходит

 multi_frame_single_stream_request проходит

 route 1/2/3 hops не ломают streaming layer

Stage 5 — Variable-length circuits
Архитектура

 один end-to-end handshake на всю цепочку

 один end-to-end AEAD layer

 relay не делает per-hop handshake

 relay не хранит per-stream state

 нет nested tunnels

 hop_index реально используется

Функциональность

 Route { hops } реально работает

 client шлёт на first_hop

 relay находит свой idx в route.hops

 relay выбирает next_hop / prev_hop

 relay инкрементит / декрементит hop_index

 exit принимает только final hop

 reverse path идёт по той же цепочке назад

E2E

 1-hop даёт HTTP 200

 2-hop даёт HTTP 200

 3-hop даёт HTTP 200

 большие ответы не дают 504

 multi-frame запросы работают через multi-hop

Производительность

 1-hop не хуже Stage 4 заметно

 2-hop overhead разумный

 3-hop работает без деградации “всё сломалось”

 нет лишнего буферинга на relay

Сквозной чек-лист после любого изменения

Вот короткая версия, которую удобно гонять каждый раз.

Архитектурные инварианты

 relay blind

 exit = единственная точка decrypt/target connect

 один end-to-end session layer

 один end-to-end reliable stream layer

 нет per-hop crypto/session

 tcp mode не сломан

 tun mode не сломан в своём scope

 route memory не выпилена

 transport abstraction не выпилена

Основные сценарии

 handshake проходит

 curl даёт HTTP 200

 unavailable target даёт ожидаемую ошибку

 multi-frame request работает

 parallel streams работают

 large transfer работает

 1-hop работает

 2-hop работает

 3-hop работает

Безопасность

 dev PSK не основной runtime path

 replay protection включена

 cookie challenge работает

 invalid handshake отклоняется

Стабильность

 нет 504 в нормальных сценариях

 нет truncation больших ответов

 нет packet storm

 нет вечного retransmit

 нет premature stream close