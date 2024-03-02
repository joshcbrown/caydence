# caydence

caydence is a small utility built on top of [swww](https://github.com/LGFae/swww).
the main goal is to switch wallpapers at specific intervals.
it switches the wallpapers in two manners:

- at regular intervals. the default is every 20 minutes, which i use as a reminder
  to look away from the screen;
- according to the [pomodoro](https://en.wikipedia.org/wiki/Pomodoro_Technique) method.
  - wallpaper switches are announced via libnotify when in this mode.

## usage

to get started, run `caydence daemon <wallpaper directory>`. this will prompt the daemon to switch wallpapers every 20 minutes.
you may want to pipe stdout to a log file, e.g. `caydence daemon &> /tmp/caydence.log`.

run one of the query commands via `caydence client`, e.g.,
`caydence client toggle` to switch to pomodoro mode. `caydence` will respond to client commands via libnotify.

there are a number of customisation options available; run `caydence help
<command> to see more`.

the idea is that the daemon is started in a launch script on your wm/dm, and the
client commands are given keybinds. in my sway config, i have:

```
exec-once ~/.cargo/bin/caydence
bindsym $mod+p exec ~/.cargo/bin/caydence client toggle
bindsym $mod+x exex ~/.cargo/gin/caydence client skip
bindsym $mod+t exec ~/.cabal/bin/caydcence client time
```

## install

caydence requires [swww](https://github.com/LGFae/swww) to run (and hence, can
only be run on wayland). an error message
will be printed if it isn't found on the path when running the daemon.

to install, run `cargo install caydence`.
