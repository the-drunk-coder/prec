# prec - The Perpetual Recorder

The `Perperual Recorder` is a small program, to, as the name says it, perpetually
record audio. 

It perpetually keeps an audio buffer of a certain length, say, on minute. Unless you tell it to keep the recorded audio, it'll just be overwritten. If you tell it to do so, it'll keep the buffer and continue recording until you tell it to stop.

It's meant to be run on a small, portable device, such as a home-made field recorder using a Raspberry PI or the like. That way, once you hear something cool, you don't have to get the microphone out because you're already recording, and you can keep what you just heard.
