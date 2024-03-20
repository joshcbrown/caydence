# caydence

caydence is a small utility built on top of [swww](https://github.com/LGFae/swww).
the main goal is to switch wallpapers at specific intervals.
it switches the wallpapers in two manners:

- at regular intervals. the default is every 20 minutes, which i use as a reminder
  to look away from the screen;
- according to the [pomodoro](https://en.wikipedia.org/wiki/Pomodoro_Technique) method.
  - wallpaper switches are announced via libnotify when in this mode.

## disclaimer

the tool is best used when you have your wallpaper in sight as you work. for those that don't use gaps/blur,
i suggest looking elsewhere for a dedicated timer app (might i suggest [pogodoro](https://github.com/joshcbrown/pogodoro)? :P)

## usage

https://github.com/joshcbrown/caydence/assets/80245312/3a3f5774-7e48-468b-9ed8-a43b927ca84f

to get started, run `caydence daemon <wallpaper directory>`. this will prompt the daemon to switch wallpapers every 20 minutes.
you may want to pipe stdout to a log file, e.g. `caydence daemon &> /tmp/caydence.log`.

run one of the query commands via `caydence client`, e.g.,
`caydence client toggle` to switch to pomodoro mode. `caydence` will respond to client commands via libnotify.

there are a number of customisation options available; run `caydence help
<command> to see more`.

the idea is that the daemon is started in a launch script on your wm/dm, and the
client commands are given keybinds. in my sway config, i have:

```
exec swww init
exec ~/.cargo/bin/caydence daemon ~/.config/sway/wallpapers/

bindsym $mod+p exec ~/.cargo/bin/caydence client toggle
bindsym $mod+x exec ~/.cargo/bin/caydence client skip
bindsym $mod+t exec ~/.cargo/bin/caydence client time
```

## install

caydence requires [swww](https://github.com/LGFae/swww) to run (and hence, can
only be run on wayland). an error message
will be printed if it isn't found on the path when running the daemon.

to install, run `cargo install caydence`.

### nix flake

`caydence` is a nix flake, so if you're running flakes on nixOS you can install it via
adding it to your inputs:
```nix
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    #...
    caydence.url = "github:joshcbrown/caydence";
  };
```

and then adding it to a list of packages via `caydence.packages.${pkgs.system}.default`.

i'm pretty sure this is total overkill, but i'm having fun so
