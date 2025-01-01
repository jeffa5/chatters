# Chatters

This repo aims to build terminal UIs (TUIs) for a few instant messaging applications that I use.
Its partly a chance to play around with different interfaces for my messages, given that I currently enjoy managing email with aerc.
Its also a chance for me to be more focused on plaintext and build a vim-like chat interface around that, after all instant messaging is _mostly_ text.
For non-text messages I expect that I'll have something like aerc's opener system where you can specify how to open filetypes, but that's difficult, more likely is just using something like xdg-open.

Another aspect of this project is to use the same core UI for different chat platforms, after all WhatsApp, Signal, and others all seem rather similar but have different UIs, maybe the configuration will be different but probably not much else.
I think this is cool as it means that you can get into more of a sense of flow across the different platforms.

Since its in Rust a core piece will be a central trait for the _chat backends_.
