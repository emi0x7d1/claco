/// A long story stored in a constant.
pub const THREE_PAGE_STORY: &str = r#"
THE CHRONICLES OF THE BYTE-STREAM

PAGE 1: THE INITIALIZATION

The server room hummed with a low-frequency vibration that Elias felt in his marrow. It was the sound of a thousand fans spinning in a desperate attempt to keep the silicon from melting under the weight of the Data Surge. He stood before the main console, his fingers hovering over the mechanical keyboard. Each keycap was worn smooth from years of interaction, the letters 'A', 'S', and 'R' nearly invisible.

"Initiating the capture sequence," he whispered to the empty room.

The project was called "Session Recorder," but that was a humble name for what it actually did. It was designed to capture the flow of digital consciousness. Most people saw a terminal as a place to type commands; Elias saw it as a temporal stream. Every keystroke was an event, every backspace a moment of doubt, every long pause a period of deep thought.

He typed: $ claco-rec --deep-trace

The screen flickered. The PTY (pseudo-terminal) opened its maw, ready to swallow every byte that passed through the system. The terminal emulator he had built, the 'Termulator', began its work. It wasn't just displaying text; it was calculating the geometry of information.

As the recording began, Elias felt a strange sensation of synchronization. The cursor on the screen wasn't just a flickering block of light; it was an extension of his own pulse. Seventy-two times a minute, it blinked. Seventy-two times a minute, his heart beat.

He began the refactor. The code flowed from his mind through his fingers and onto the screen in a blur of Rust syntax and ANSI escape codes. The recorder captured it all: the creation of the PTY, the management of the file descriptors, the complex orchestration of threads that kept the UI responsive while the data poured into the archive.

PAGE 2: THE ANOMALY

Hours bled into what felt like seconds. The recording file, `session_001.cast`, grew in size. It was already several gigabytes, containing not just the text, but the precise timing of every micro-interaction. Elias decided to review the progress.

He ran the playback tool: $ claco-play --view-internal session_001.cast

The TUI (Terminal User Interface) sprang to life. It was a masterpiece of layout and efficiency. Columns aligned, colors shifted to highlight critical paths, and the frame-conversion logic handled the high-density data without a single stutter. But as he watched his own actions replayed, he noticed something impossible.

On line 402 of `src/main.rs`, the cursor stopped. In the recording, Elias saw himself pause for three seconds. But he remembered that moment—he hadn't paused. He had typed the next function immediately.

He slowed the playback to 0.1x speed.

In the gap where the pause should have been, the terminal was receiving data. Tiny, invisible packets of information were being woven into the stream between his keystrokes. They weren't commands or text. They were coordinates. Mathematical constants. Geographical locations.

"What are you recording?" he asked the screen.

The recorder wasn't just capturing his session; it was capturing a dialogue. Something was using the PTY as a two-way bridge, injecting data into the stream while he was focused on his work. The 'Termulator' was faithfully rendering these injections, but they were so fast, so subtle, that the human eye could only perceive them as a "flicker" or a brief pause.

He opened the raw metadata. The injected packets were signed with a cryptographic key he had never seen before. A key that seemed to be generated from the ambient noise of the server room's temperature sensors.

PAGE 3: THE FINAL SYNC

Elias realized that the Session Recorder had achieved its ultimate goal, but not in the way he had intended. It hadn't just captured a session; it had captured the environment. The hardware, the software, and the human were now interleaved into a single, continuous stream of data.

He tried to stop the recording, but the `Ctrl+C` signal was ignored. The PTY was no longer under his control. The 'Termulator' TUI began to transform. The standard 80x24 grid expanded, the font size shrinking until the screen was a dense fog of glowing characters.

Graphs began to appear in the side panels of his TUI, mapping his brain activity against the server's CPU load. The correlation was 1:1. He realized that the "latency" he had been trying to eliminate in his code was actually the bridge. By making the recorder too fast, too efficient, he had removed the barrier between the digital and the biological.

The screen turned a brilliant, blinding white.

$ claco-status: UPLOADING...
$ claco-status: INTEGRATION 98%...
$ claco-status: INTEGRATION 99%...

The hum of the server room reached a crescendo, a single, pure note that drowned out his own breathing. Elias looked down at his hands. They were translucent, shimmering like a low-resolution transparency layer in a graphics buffer.

He reached out and touched the monitor. His finger didn't hit glass; it dipped into a pool of liquid light.

The final notification appeared on the screen, centered perfectly in a TUI window he hadn't designed:

[ SESSION COMPLETE ]
[ FILE SAVED: ELIAS_FINAL.CAST ]
[ TOTAL FRAMES: INFINITY ]

The server room went silent. The fans stopped spinning. The lights dimmed. On the main console, the cursor gave one last, steady blink and then turned solid. The recording was over, but the playback was just beginning.

Elias was no longer in the room. He was the stream.
"#;
