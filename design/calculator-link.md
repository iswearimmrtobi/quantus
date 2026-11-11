# Calculator link: contacts, sending and two-player games

Two handhelds that are held together become each other's **contacts**. From then on either can
send the other a session, as AirDrop does, and the two can play two-player games, four in a row
first. This is an idea, not planned work; the short form is idea 38 in
[feature-ideas.md](../plans/feature-ideas.md).

- **Section 1**: goal and scope.
- **Section 2**: the NFC hardware.
- **Section 3–5**: identity, pairing, contacts.
- **Section 6–7**: transport and messages.
- **Section 8–9**: sending sessions, games.
- **Section 10–11**: UI, exam mode and privacy.
- **Section 12–13**: phases, open questions.

State on 2026-11-10.

## 1. Goal and scope

- Calculator to calculator only, with no access point, server or phone in between.
- The planned link between handheld and desktop ([plans.md](../plans/plans.md) §5) is separate.
  It can later use the same identity and contacts, so a desktop can be a contact too (§12 e).
- Everything that is received needs a yes on a popup. Nothing arrives silently.

## 2. Hardware: NFC

The board has no NFC today. The C6's radio reaches across a room, so it cannot tell that two
calculators are touching; a tap needs a reader with a range of a few centimetres.

- **Chip**: NXP **PN7160**, on I²C with an IRQ and an enable (VEN) pin to the P4. It speaks NCI
  2.0 and can poll as a reader and listen as a card, which is what two equal devices need (§4).
  The alternative is ST's ST25R3916, which is more flexible and needs more driver work.
- **Antenna**: a coil on the back of our PCB, near the top edge, so the two calculators are held
  back to back or top to top. No copper pour under the coil. If the battery sits behind it, a
  ferrite sheet goes between them. The matching network (capacitors from the chip's application
  note) is tuned on the first board.
- **Power**: VEN is low unless the *Kontakt hinzufügen* screen is open. No background polling at
  first; a low duty-cycle poll can come later if the tap should work from any screen.
- **Exam mode**: VEN stays low (§11).
- **Open**: free I²C bus and GPIOs (check against [hardware.md](../firmware/docs/hardware.md)),
  board space for the coil, BOM cost.

## 3. Identity

- On first boot each handheld makes an **Ed25519 key pair** and keeps it in NVS. The secret key
  never leaves the device, and it is not on the SD card.
- Its **ID** is the first 8 bytes of the public key, shown as 4 groups of 4 hex digits in
  SETUP → Gerät.
- The user picks a **name** (default: *Quantus* + the last 4 digits of the ID) and a **colour**
  from the palette in SETUP. The colour makes a contact easy to spot in lists and popups.
- Resetting the device makes a new key pair; old contacts then no longer accept it, and it must
  be added again.

## 4. Pairing over NFC

1. Both users open Kontakte → *Kontakt hinzufügen*. The NFC loop alternates between polling
   (reader) and listening (card emulation, Type 4 tag). The device that polls first is the
   reader; NFC discovery settles this by itself.
2. The reader selects the other's application and the two exchange a **pairing record** each way
   in APDUs: format version, public key, name, colour, the C6's MAC for ESP-NOW, a random nonce.
3. Both screens show *Kontakt hinzufügen: ‹name›?* in the other's colour, with **OK** and
   **Abbrechen**. The calculators can now be taken apart.
4. When a user presses OK, their device sends a `PairConfirm` over ESP-NOW (§6), signed with its
   key over both nonces. The contact is stored only when both sides confirmed, so a one-sided add
   does not exist. Without the other's confirm within 60 s, the screen says so and stores nothing.

The keys are exchanged physically, at a few centimetres, so there are no PINs or codes to compare.

## 5. Contacts

- Kept in `/contacts.json` on the SD card, with a `version` field, read like the session files
  (an older version is brought up to date, a newer one refused).
- Per contact: public key, name, colour, MAC, date added, and a name of one's own that overrides
  theirs.
- Kontakte can rename and delete a contact. Deleting is local; the other side keeps its entry, but
  its messages are dropped (§6).
- Without an SD card, contacts are kept in NVS instead, limited to 16.

## 6. Transport

- **ESP-NOW** on the C6: connectionless frames between MACs, with no access point and the range
  of a classroom. It works on the Wi-Fi channel the C6 is on; without WLAN the two devices use
  a fixed channel (1).
- The C6 runs ESP-Hosted's co-processor firmware ([wlan.md](../firmware/docs/wlan.md)). Its
  ESP-NOW API has to reach the P4. Either ESP-Hosted forwards it, or `firmware/c6` gets a few
  custom RPCs: `espnow_start(channel)`, `espnow_stop`, `send(mac, bytes)`, and a receive event.
  Which one, is the first thing to find out (§13).
- Frames carry at most 1470 bytes (ESP-NOW v2; 250 with v1). Larger payloads are cut into
  numbered chunks; each chunk is acked, and a missing ack resends it, 3 times at most.
- **Encryption**: both sides derive a shared key with X25519 from the Ed25519 keys and seal every
  message with ChaCha20-Poly1305. The nonce holds a counter that only goes up, so a replayed frame
  is dropped.
- Frames from a MAC or key that is not a contact are dropped without an answer, except during
  pairing (§4).

## 7. Messages

Each message is a small binary envelope: `type` (1 byte), `seq` (u32), `payload`.

| Type | Payload | Use |
|---|---|---|
| `Presence` | name, colour | "I am here", every 5 s while visible (§11) |
| `PairConfirm` | signature over both nonces | §4 |
| `Offer` | kind (session, …), name, size, hash | asks the other to accept a file |
| `Accept` / `Decline` | offer `seq` | the answer |
| `Chunk` | offer `seq`, index, bytes | the data |
| `Ack` | `seq`, index | for each chunk and each game move |
| `Done` | offer `seq` | all chunks sent; the receiver checks the hash |
| `GameInvite` | game, rules version | asks to play |
| `GameAccept` / `GameDecline` | invite `seq` | the answer |
| `Move` | game id, move number, move | one move |
| `Resign` / `Rematch` | game id | end, or play again |

## 8. Sending a session

1. Sitzungen → a session → **Senden** → a contact from the list, those that are visible first.
2. The receiver sees *‹name› sendet Sitzung „‹session›“ (12 kB)* with **Annehmen** and
   **Ablehnen**. Without an answer within 60 s, the sender sees *Nicht angenommen*.
3. The data is the session JSON from `quantus-core` (`Session::to_json`, read with
   `Session::from_json` in `crates/quantus-core/src/session.rs`, which carries `FORMAT_VERSION`).
   A newer format is refused with the core's message, as when opening a file from the card.
4. The received session goes into the sessions folder on the card, with the sender's name in its
   name if one of that name exists already, and opens in Sitzungen.

The same path can later send single results, the packs of idea 19 and the worksheets of idea 25:
each is one more `kind` in `Offer`.

## 9. Two-player games

**Four in a row** first: a 7 × 6 board, drawn in the two players' colours.

- The game is a pure module in `firmware/ui` (board, legal moves, win and draw check), with
  tests, and no knowledge of the radio. The hui sends and receives `Move`s for it.
- The inviter plays first; with *Rematch*, the other one starts.
- Each side checks every move it receives. An illegal move ends the game with a message.
- A move that arrives twice is ignored by its move number; one that is not acked is resent.
- If the other side is silent for 30 s, the game shows *Verbindung verloren*. The game state is
  kept in RAM, so the two can go on when the other comes back, until one of them leaves the game.
- Keys: ◄ ► choose the column, OK drops the piece, AC offers to resign.

Later games on the same messages: battleships, a maths duel (both get the same task, the faster
correct result scores).

## 10. UI

- A new tile **Kontakte** (or an entry in SETUP → Verbindungen): the list with name, colour and
  whether the contact is visible; *Kontakt hinzufügen*; details (ID, date, rename, delete).
- From a contact: *Sitzung senden*, *Vier gewinnt spielen*.
- Popups for pairing, incoming offers and game invites, over whatever screen is open.
- A top-bar icon while ESP-NOW is on, next to the Wi-Fi fan and the Bluetooth rune.
- All texts go into every file of `locales/`.

## 11. Exam mode and privacy

- **Prüfungsmodus** keeps NFC and ESP-NOW off and drops anything received, like WLAN today. Games
  are not reachable in it.
- **Visibility** is opt-in: a device answers and sends `Presence` only while Kontakte or a game is
  open, or when *Sichtbar für Kontakte* is on in SETUP → Verbindungen (off by default).
- Nothing is stored or started without a yes on a popup.

## 12. Phases

1. **a** Identity, contacts file, ESP-NOW between two dev kits. Pairing by a test menu that
   swaps the records over the serial console instead of NFC.
2. **b** Sending sessions.
3. **c** Four in a row.
4. **d** NFC on the next PCB revision (PN7160 and coil), then real pairing.
5. **e** The desktop as a contact: the handheld has no camera, so it shows its pairing record as
   a QR code and the desktop reads it with a webcam, or the user types the ID. The desktop's
   record goes back over the link of plans.md §5.

## 13. Open questions

- Whether ESP-Hosted 3.0.x forwards ESP-NOW to the host, or `firmware/c6` needs its own RPCs.
- Whether ESP-NOW and a WLAN connection run together well on the C6 (same channel, so two
  calculators on different networks cannot reach each other).
- PN7160 or ST25R3916; antenna placement and range through the case.
- A size limit for one transfer (a session is a few kB; packs may be larger).
- Whether games on a school calculator are welcome, or should be something a teacher can switch
  off (see idea 24's exam profiles).
