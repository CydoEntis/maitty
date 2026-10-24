# Theme sources

Research checked on 2026-10-14 against the creators' source repositories. The
catalog covers all 25 requested families with 42 palettes. Before this expansion,
Seshi already included Dracula, Gruvbox Dark, Nord, Tokyo Night Night, and
Catppuccin Mocha/Latte. These are six palettes across five families.

A second pass on 2026-10-23 added 43 palettes from 35 more creator repositories
(Nightfox through Bluloco in the table). Everviolet was left out: it is an
Everforest-style green theme that duplicates Everforest.

## Coverage and primary sources

Source links pin the researched revision. Preserve the existing `gruvbox` and
`tokyo-night` configuration names for their dark/night palettes.

| Family | Coverage | Included variants | Palette source | License |
| --- | --- | --- | --- | --- |
| Dracula | Existing | Standard | [dracula/alacritty](https://github.com/dracula/alacritty/blob/c8a3a13404b78d520d04354e133b5075d9b785e1/dracula.toml) | [MIT](https://github.com/dracula/alacritty/blob/c8a3a13404b78d520d04354e133b5075d9b785e1/LICENSE) |
| Gruvbox | Dark existing; light added | Dark / Light, medium contrast | [morhetz/gruvbox](https://github.com/morhetz/gruvbox/blob/ef8864bb42bf244f0295d1c5a403b27e3d139695/colors/gruvbox.vim) | [MIT/X11](https://github.com/morhetz/gruvbox/blob/ef8864bb42bf244f0295d1c5a403b27e3d139695/README.md#license) |
| Nord | Existing | Standard | [nordtheme/alacritty](https://github.com/nordtheme/alacritty/blob/9949642f3903e8fcb62bfc03f09410e3d78440c2/src/nord.yaml) | [MIT](https://github.com/nordtheme/alacritty/blob/9949642f3903e8fcb62bfc03f09410e3d78440c2/license) |
| Tokyo Night | Night existing; variants added | Night / Storm / Moon / Day | [folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim/tree/cdc07ac78467a233fd62c493de29a17e0cf2b2b6/extras/alacritty) | [Apache-2.0](https://github.com/folke/tokyonight.nvim/blob/cdc07ac78467a233fd62c493de29a17e0cf2b2b6/LICENSE) |
| Catppuccin | Mocha and Latte existing; flavors added | Latte / Frappe / Macchiato / Mocha | [catppuccin/alacritty](https://github.com/catppuccin/alacritty/blob/f6cb5a5c2b404cdaceaff193b9c52317f62c62f7/catppuccin-frappe.toml) | [MIT](https://github.com/catppuccin/alacritty/blob/f6cb5a5c2b404cdaceaff193b9c52317f62c62f7/LICENSE) |
| Solarized | Added | Dark / Light | [altercation/solarized](https://github.com/altercation/solarized/blob/62f656a02f93c5190a8753159e34b385588d5ff3/README.md#the-values) | [MIT](https://github.com/altercation/solarized/blob/62f656a02f93c5190a8753159e34b385588d5ff3/LICENSE) |
| One Dark / Atom One | Added | One Dark | [joshdick/onedark.vim](https://github.com/joshdick/onedark.vim/blob/47bec7a6196a843dad195d2666c3ac84c6e80c78/term/One%20Dark.alacritty) | [MIT](https://github.com/joshdick/onedark.vim/blob/47bec7a6196a843dad195d2666c3ac84c6e80c78/LICENSE) |
| One Half | Added | Dark / Light | [sonph/onehalf](https://github.com/sonph/onehalf/tree/75eb2e97acd74660779fed8380989ee7891eec56/kitty) | [MIT](https://github.com/sonph/onehalf/blob/75eb2e97acd74660779fed8380989ee7891eec56/LICENSE.txt) |
| Kanagawa | Added | Wave / Dragon / Lotus | [rebelot/kanagawa.nvim](https://github.com/rebelot/kanagawa.nvim/tree/bb85e4bfc8d89b0e62c8fa53ccdd13d12e2f77b3/extras/alacritty) | [MIT](https://github.com/rebelot/kanagawa.nvim/blob/bb85e4bfc8d89b0e62c8fa53ccdd13d12e2f77b3/LICENSE) |
| Ayu | Added | Dark / Mirage / Light | [ayu-theme/ayu-vim](https://github.com/ayu-theme/ayu-vim/blob/01faacb4cb76e8cf72ad9858c581d80876260ab3/colors/ayu.vim) | [Apache-2.0](https://github.com/ayu-theme/ayu-vim/blob/01faacb4cb76e8cf72ad9858c581d80876260ab3/LICENSE) |
| Night Owl | Added | Night Owl dark | [sdras/night-owl-vscode-theme](https://github.com/sdras/night-owl-vscode-theme/blob/cc291eba7976b20d7c66bde6883c27b902196b07/themes/Night%20Owl-color-theme.json) | [MIT](https://github.com/sdras/night-owl-vscode-theme/blob/cc291eba7976b20d7c66bde6883c27b902196b07/LICENSE.md) |
| Nightfly | Added | Standard | [bluz71/vim-nightfly-colors](https://github.com/bluz71/vim-nightfly-colors/blob/2c31d07ce05efa92bcb7f893f9c74a846226c898/extras/nightfly-alacritty.toml) | [MIT](https://github.com/bluz71/vim-nightfly-colors/blob/2c31d07ce05efa92bcb7f893f9c74a846226c898/LICENSE) |
| Moonfly | Added | Standard | [bluz71/vim-moonfly-colors](https://github.com/bluz71/vim-moonfly-colors/blob/2c883835f5b7b380a0a9e80695c420f7efebdfe4/extras/moonfly-alacritty.toml) | [MIT](https://github.com/bluz71/vim-moonfly-colors/blob/2c883835f5b7b380a0a9e80695c420f7efebdfe4/LICENSE) |
| Everforest | Added | Dark / Light, medium contrast | [sainnhe/everforest](https://github.com/sainnhe/everforest/blob/85a86eb62409e3ec88713bff3d1b9d7374e112e4/autoload/everforest.vim) | [MIT](https://github.com/sainnhe/everforest/blob/85a86eb62409e3ec88713bff3d1b9d7374e112e4/LICENSE) |
| Rose Pine | Added | Base / Moon / Dawn | [rose-pine/alacritty](https://github.com/rose-pine/alacritty/tree/a6f7c8245e5bba639befe52e1e025f84ba8b3ee5/dist) | [MIT](https://github.com/rose-pine/alacritty/blob/a6f7c8245e5bba639befe52e1e025f84ba8b3ee5/LICENSE) |
| Cobalt2 | Added | Creator's VS Code terminal palette | [wesbos/cobalt2-vscode](https://github.com/wesbos/cobalt2-vscode/blob/c4e9574372b85afad1682ed0fdd1ac0411c62512/theme/cobalt2.json) | [MIT](https://github.com/wesbos/cobalt2-vscode/blob/c4e9574372b85afad1682ed0fdd1ac0411c62512/LICENSE) |
| Jellybeans | Added | Standard | [nanotech/jellybeans.vim](https://github.com/nanotech/jellybeans.vim/blob/ef83bf4dc8b3eacffc97bf5c96ab2581b415c9fa/ansi-term-colors.txt) | [MIT](https://github.com/nanotech/jellybeans.vim/blob/ef83bf4dc8b3eacffc97bf5c96ab2581b415c9fa/colors/jellybeans.vim) |
| Iceberg | Added | Dark | [cocopon/iceberg.vim](https://github.com/cocopon/iceberg.vim/blob/23835d5ed696436f716cbfdb56a93a7850fe3b18/colors/iceberg.vim) | [MIT](https://github.com/cocopon/iceberg.vim/blob/23835d5ed696436f716cbfdb56a93a7850fe3b18/LICENSE.txt) |
| Sonokai | Added | Default | [sainnhe/sonokai](https://github.com/sainnhe/sonokai/blob/b023c5280b16fe2366f5e779d8d2756b3e5ee9c3/autoload/sonokai.vim) | [MIT](https://github.com/sainnhe/sonokai/blob/b023c5280b16fe2366f5e779d8d2756b3e5ee9c3/LICENSE) |
| Shades of Purple | Added | Standard terminal palette | [ahmadawais/shades-of-purple-vscode](https://github.com/ahmadawais/shades-of-purple-vscode/blob/e8eb49f33e5db05ceba6677367b33ddb27ad821c/themes/shades-of-purple-color-theme.json) | [MIT](https://github.com/ahmadawais/shades-of-purple-vscode/blob/e8eb49f33e5db05ceba6677367b33ddb27ad821c/LICENSE.md) |
| Panda | Added | Original Panda Syntax | [siamak/atom-panda-syntax](https://github.com/siamak/atom-panda-syntax/blob/5f8da63f050a79682253630639a77aba93cb1430/styles/colors.less) | [MIT](https://github.com/siamak/atom-panda-syntax/blob/5f8da63f050a79682253630639a77aba93cb1430/LICENSE.md) |
| Lucario | Added | Standard | [raphamorim/lucario](https://github.com/raphamorim/lucario/blob/a9cf850453268d3e62d16516f92d399d3587058a/alacritty/lucario.toml) | [MIT](https://github.com/raphamorim/lucario/blob/a9cf850453268d3e62d16516f92d399d3587058a/LICENSE) |
| Gotham | Added | Standard | [whatyouhide/vim-gotham](https://github.com/whatyouhide/vim-gotham/blob/7e7eff411fedada2b6cceb78059ed8a4a26d03ba/colors/gotham.vim) | [MIT](https://github.com/whatyouhide/vim-gotham/blob/7e7eff411fedada2b6cceb78059ed8a4a26d03ba/LICENSE.txt) |
| Apprentice | Added | Standard | [romainl/Apprentice](https://github.com/romainl/Apprentice/blob/2a4557410fe6b5e3906c5290d42c4e4d8813b0d8/colortemplate/apprentice.colortemplate) | [MIT](https://github.com/romainl/Apprentice/blob/2a4557410fe6b5e3906c5290d42c4e4d8813b0d8/LICENSE) |
| Zenbones | Added | Dark / Light, default contrast | [zenbones-theme/zenbones.nvim](https://github.com/zenbones-theme/zenbones.nvim/tree/8304d8df9b823ff11e103afa62f38c39f534abe6/extras/alacritty) | [MIT](https://github.com/zenbones-theme/zenbones.nvim/blob/8304d8df9b823ff11e103afa62f38c39f534abe6/LICENSE) |
| Nightfox | Added | Nightfox / Duskfox / Carbonfox | [EdenEast/nightfox.nvim](https://github.com/EdenEast/nightfox.nvim/blob/4dacd3f0185a2227bdf3b6c0975a8f0bf87cac9a/extra/nightfox/kitty.conf) | [MIT](https://github.com/EdenEast/nightfox.nvim/blob/4dacd3f0185a2227bdf3b6c0975a8f0bf87cac9a/LICENSE) |
| GitHub | Added | GitHub Dark / GitHub Dark Dimmed / GitHub Light | [projekt0n/github-nvim-theme](https://github.com/projekt0n/github-nvim-theme/blob/c106c9472154d6b2c74b74565616b877ae8ed31d/lua/github-theme/palette/github_dark_default.lua) | [MIT](https://github.com/projekt0n/github-nvim-theme/blob/c106c9472154d6b2c74b74565616b877ae8ed31d/LICENSE) |
| Gruvbox Material | Added | Standard | [sainnhe/gruvbox-material](https://github.com/sainnhe/gruvbox-material/blob/11d779b26a9ab2b3db8c22c6ac9fb6e8ed4fea79/autoload/gruvbox_material.vim) | [MIT](https://github.com/sainnhe/gruvbox-material/blob/11d779b26a9ab2b3db8c22c6ac9fb6e8ed4fea79/LICENSE) |
| Gruvbox Baby | Added | Standard | [luisiacc/gruvbox-baby](https://github.com/luisiacc/gruvbox-baby/blob/bd52e62d8134647090108189e69c8b3cd18bdbbf/extras/windwows_terminal/windows_terminal_dark.json) | [MIT](https://github.com/luisiacc/gruvbox-baby/blob/bd52e62d8134647090108189e69c8b3cd18bdbbf/LICENSE) |
| Oceanic Next | Added | Standard | [mhartington/oceanic-next](https://github.com/mhartington/oceanic-next/blob/09833f72d5ba23de2e8bcae18f479f326f5f677a/colors/OceanicNext.vim) | [Apache-2.0](https://github.com/mhartington/oceanic-next/blob/09833f72d5ba23de2e8bcae18f479f326f5f677a/license.txt) |
| Tomorrow | Added | Tomorrow / Tomorrow Night / Tomorrow Night Eighties / Tomorrow Night Bright | [chriskempson/tomorrow-theme](https://github.com/chriskempson/tomorrow-theme/blob/ccf6666d888198d341b26b3a99d0bc96500ad503/iTerm2/Tomorrow.itermcolors) | [MIT](https://github.com/chriskempson/tomorrow-theme/blob/ccf6666d888198d341b26b3a99d0bc96500ad503/LICENSE.md) |
| Base16 | Added | Standard | [tinted-theming/schemes](https://github.com/tinted-theming/schemes/blob/3ac91137e879502c442ab89ba0f1a98077d0da9d/base16/default-dark.yaml) | [MIT](https://github.com/tinted-theming/schemes/blob/3ac91137e879502c442ab89ba0f1a98077d0da9d/LICENSE) |
| Palenight | Added | Standard | [drewtempelmeyer/palenight.vim](https://github.com/drewtempelmeyer/palenight.vim/blob/9e9b102c54b830b72f51917b6b86c3b3b02dcea2/autoload/palenight.vim) | [MIT](https://github.com/drewtempelmeyer/palenight.vim/blob/9e9b102c54b830b72f51917b6b86c3b3b02dcea2/LICENSE) |
| One Light | Added | Standard | [rakr/vim-one](https://github.com/rakr/vim-one/blob/187f5c85b682c1933f8780d4d419c55d26a82e24/colors/one.vim) | [MIT](https://github.com/rakr/vim-one/blob/187f5c85b682c1933f8780d4d419c55d26a82e24/LICENSE) |
| Monokai Pro | Added | Standard | [monokai-pro/zed](https://github.com/monokai-pro/zed/blob/7b4ebf709abe9870a3fc9bafc1e80796e5f0e97c/themes/monokai-pro-ce.json) | [MIT](https://github.com/monokai-pro/zed/blob/7b4ebf709abe9870a3fc9bafc1e80796e5f0e97c/LICENSE.md) |
| Oxocarbon | Added | Standard | [nyoom-engineering/oxocarbon.nvim](https://github.com/nyoom-engineering/oxocarbon.nvim/blob/cd6523a0836d6e8ee823d343149fd06c7b71fdde/lua/oxocarbon/init.lua) | [MIT](https://github.com/nyoom-engineering/oxocarbon.nvim/blob/cd6523a0836d6e8ee823d343149fd06c7b71fdde/LICENSE) |
| Cyberdream | Added | Standard | [scottmckendry/cyberdream.nvim](https://github.com/scottmckendry/cyberdream.nvim/blob/81b3e39ceb87db2987255123af3212beeb75bf6d/extras/alacritty/cyberdream.toml) | [MIT](https://github.com/scottmckendry/cyberdream.nvim/blob/81b3e39ceb87db2987255123af3212beeb75bf6d/LICENSE) |
| Nordic | Added | Standard | [AlexvZyl/nordic.nvim](https://github.com/AlexvZyl/nordic.nvim/blob/9fb62116b891184049c50ca6ffb0d3bc100a1a6e/platforms/kitty/nordic.conf) | [MIT](https://github.com/AlexvZyl/nordic.nvim/blob/9fb62116b891184049c50ca6ffb0d3bc100a1a6e/LICENSE) |
| OneNord | Added | Standard | [rmehri01/onenord.nvim](https://github.com/rmehri01/onenord.nvim/blob/5240bfb79bcab7e03508626c5283cfc417b79169/extras/kitty/onenord.conf) | [MIT](https://github.com/rmehri01/onenord.nvim/blob/5240bfb79bcab7e03508626c5283cfc417b79169/LICENSE) |
| Edge | Added | Standard | [sainnhe/edge](https://github.com/sainnhe/edge/blob/7ec78c21bd8f0b2b0b47043bb1e730c561647c72/autoload/edge.vim) | [MIT](https://github.com/sainnhe/edge/blob/7ec78c21bd8f0b2b0b47043bb1e730c561647c72/LICENSE) |
| VSCode | Added | Standard | [Mofiqul/vscode.nvim](https://github.com/Mofiqul/vscode.nvim/blob/6439ed89d0e10193f07e92268a321d7c6f6e93ee/extra/kitty/vscode-dark.conf) | [MIT](https://github.com/Mofiqul/vscode.nvim/blob/6439ed89d0e10193f07e92268a321d7c6f6e93ee/LICENSE.md) |
| Code Dark | Added | Standard | [tomasiser/vim-code-dark](https://github.com/tomasiser/vim-code-dark/blob/4624dc223bf574aa9a731c2383c579847303c380/colors/codedark.vim) | [MIT](https://github.com/tomasiser/vim-code-dark/blob/4624dc223bf574aa9a731c2383c579847303c380/LICENSE.md) |
| Melange | Added | Standard | [savq/melange-nvim](https://github.com/savq/melange-nvim/blob/ce42f6b629beeaa00591ba73a77d3eeac4cf28ce/term/kitty/melange_dark.conf) | [MIT](https://github.com/savq/melange-nvim/blob/ce42f6b629beeaa00591ba73a77d3eeac4cf28ce/LICENSE) |
| Falcon | Added | Standard | [fenetikm/falcon](https://github.com/fenetikm/falcon/blob/276cd483c73f5389f0bd99b18e2452bd57b77bf6/kitty/kitty.falcon.conf) | [MIT](https://github.com/fenetikm/falcon/blob/276cd483c73f5389f0bd99b18e2452bd57b77bf6/LICENSE) |
| Embark | Added | Standard | [embark-theme/vim](https://github.com/embark-theme/vim/blob/2331671b548abcade22b0197fde58e11744c99bb/colors/embark.vim) | [MIT](https://github.com/embark-theme/vim/blob/2331671b548abcade22b0197fde58e11744c99bb/LICENSE) |
| Lackluster | Added | Standard | [slugbyte/lackluster.nvim](https://github.com/slugbyte/lackluster.nvim/blob/70dd682e564784893b984deb51dd5ddd263c8cc7/extra/kitty/lackluster.conf) | [MIT](https://github.com/slugbyte/lackluster.nvim/blob/70dd682e564784893b984deb51dd5ddd263c8cc7/LICENSE.md) |
| Miasma | Added | Standard | [xero/miasma.nvim](https://github.com/xero/miasma.nvim/blob/627f2e1cac91de0d1d4dd7472b506a30f41b2b7d/extras/miasma.ghostty) | [CC0-1.0](https://github.com/xero/miasma.nvim/blob/627f2e1cac91de0d1d4dd7472b506a30f41b2b7d/LICENSE) |
| Bamboo | Added | Standard | [ribru17/bamboo.nvim](https://github.com/ribru17/bamboo.nvim/blob/1309bc88bffcf1bedc3e84e7fa9004de93da774a/extras/kitty/bamboo.conf) | [MIT](https://github.com/ribru17/bamboo.nvim/blob/1309bc88bffcf1bedc3e84e7fa9004de93da774a/LICENSE) |
| Mellifluous | Added | Standard | [ramojus/mellifluous.nvim](https://github.com/ramojus/mellifluous.nvim/blob/ec0575bdb63594c5e7fba6eb9607b009aa1fb74e/lua/mellifluous/colors/colorsets/mellifluous.lua) | [MIT](https://github.com/ramojus/mellifluous.nvim/blob/ec0575bdb63594c5e7fba6eb9607b009aa1fb74e/LICENSE) |
| Mellow | Added | Standard | [mellow-theme/mellow.nvim](https://github.com/mellow-theme/mellow.nvim/blob/6273b7f0ddf6ac2b75021b3bc0bd8b3305cfeef1/lua/mellow/colors.lua) | [MIT](https://github.com/mellow-theme/mellow.nvim/blob/6273b7f0ddf6ac2b75021b3bc0bd8b3305cfeef1/LICENSE) |
| Kanagawa Paper | Added | Standard | [thesimonho/kanagawa-paper.nvim](https://github.com/thesimonho/kanagawa-paper.nvim/blob/d6ba194faba511c9b112a13c79b5e123dcb5dd41/extras/alacritty/kanagawa-paper-ink.toml) | [MIT](https://github.com/thesimonho/kanagawa-paper.nvim/blob/d6ba194faba511c9b112a13c79b5e123dcb5dd41/LICENSE) |
| Zephyr | Added | Standard | [nvimdev/zephyr-nvim](https://github.com/nvimdev/zephyr-nvim/blob/7fd86b7164442d3b5ec2c81b2694d040e716b5cf/lua/zephyr.lua) | [MIT](https://github.com/nvimdev/zephyr-nvim/blob/7fd86b7164442d3b5ec2c81b2694d040e716b5cf/LICENSE) |
| Koda | Added | Standard | [oskarnurm/koda.nvim](https://github.com/oskarnurm/koda.nvim/blob/25a5c98ca2f6bb0b69f5e2654a78f0d7c7e0721d/extras/ghostty/koda-dark.conf) | [MIT](https://github.com/oskarnurm/koda.nvim/blob/25a5c98ca2f6bb0b69f5e2654a78f0d7c7e0721d/LICENSE) |
| Aquarium | Added | Standard | [FrenzyExists/aquarium-vim](https://github.com/FrenzyExists/aquarium-vim/blob/d5c4816717e5136278a9148bd19268fcaf514fe9/extras/terminal-themes/wezterm/aquarium-dark.toml) | [MIT](https://github.com/FrenzyExists/aquarium-vim/blob/d5c4816717e5136278a9148bd19268fcaf514fe9/LICENSE) |
| Fluoromachine | Added | Standard | [maxmx03/fluoromachine.nvim](https://github.com/maxmx03/fluoromachine.nvim/blob/a279f667168e742df059caf1f1f4daf1de24f4f8/lua/fluoromachine/colors/fluoromachine.lua) | [MIT](https://github.com/maxmx03/fluoromachine.nvim/blob/a279f667168e742df059caf1f1f4daf1de24f4f8/LICENSE) |
| SynthWave '84 | Added | Standard | [robb0wen/synthwave-vscode](https://github.com/robb0wen/synthwave-vscode/blob/ecfa2fe1279f7233663fa3f98a96e6756000567b/themes/synthwave-color-theme.json) | [MIT](https://github.com/robb0wen/synthwave-vscode/blob/ecfa2fe1279f7233663fa3f98a96e6756000567b/LICENSE) |
| Horizon | Added | Standard | [jolaleye/horizon-theme-vscode](https://github.com/jolaleye/horizon-theme-vscode/blob/5ae91b6d49bf291e0a34c0a1cb277d9738aadd90/themes/horizon.json) | [MIT](https://github.com/jolaleye/horizon-theme-vscode/blob/5ae91b6d49bf291e0a34c0a1cb277d9738aadd90/LICENSE) |
| Material | Added | Darker | [marko-cerovac/material.nvim](https://github.com/marko-cerovac/material.nvim/blob/92f7366a9315cd386bc4fa4d716fde375dda210f/lua/material/colors/init.lua) | [GPL-2.0](https://github.com/marko-cerovac/material.nvim/blob/92f7366a9315cd386bc4fa4d716fde375dda210f/LICENSE) |
| Modus | Added | Vivendi / Operandi | [protesilaos/modus-themes](https://github.com/protesilaos/modus-themes/blob/b1647b2980f4f27c3c81ac1531d992a14fefd06a/modus-themes.el) | [GPL-3.0-or-later](https://github.com/protesilaos/modus-themes/blob/b1647b2980f4f27c3c81ac1531d992a14fefd06a/COPYING) |
| Bluloco | Added | Dark | [uloco/bluloco.nvim](https://github.com/uloco/bluloco.nvim/blob/912227935d63766fe11e4168840063bd2e9a4274/terminal-themes/kitty/BlulocoDark.conf) | [LGPL-3.0](https://github.com/uloco/bluloco.nvim/blob/912227935d63766fe11e4168840063bd2e9a4274/LICENSE) |

## Mapping to Seshi

These are terminal palettes adapted to Seshi's UI roles, rather than editor
syntax themes reproduced verbatim. Keep the original background, foreground,
and ANSI accent identities; derive surfaces, borders, muted labels, and status
roles for Seshi's contrast and semantic checks. Upstream terminal bright-black
colors are frequently unsuitable for small UI labels. A selection background
may assume dark selected text, so it is not automatically a valid menu fill.

- Catppuccin supplies four distinct flavors; Latte's source foreground is
  `#4c4f69`. The older Seshi Latte foreground was not the canonical Latte text.
- Tokyo Night includes the official Moon terminal variant as well as Night,
  Storm, and Day. Its Day foreground needs the UI's strongest-text contrast
  adjustment on derived surfaces.
- Gruvbox uses the source's bright accents in dark mode and faded accents in
  light mode; both use its default medium background contrast.
- Solarized uses Ethan Schoonover's canonical sRGB base and accent colors.
  Its precise palette does not imply that every accent passes Seshi's UI
  contrast thresholds unchanged.
- Kanagawa includes Wave, Dragon, and the light Lotus variant. Everforest uses
  the default medium contrast for its dark and light variants.
- Ayu uses the original creator-owned `ayu-vim` palette and explicit terminal
  assignments. The newer `ayu-colors` repository computes revised colors from
  luminance ranges; do not mix those generations within one variant.
- Cobalt2 uses Wes Bos' MIT-licensed VS Code terminal palette. His original
  iTerm repository has no standalone license; its values also differ from this
  official port. The terminal background is `#122738`.
- Panda's named colors are verified against the original author's MIT-licensed
  Atom theme. Its terminal slot mapping is also verified in
  [PandaTheme's VS Code source](https://github.com/PandaTheme/panda-syntax-vscode/blob/f704f5af0907e1320b224757171023cb05c5734f/themes/workbench.yaml).
  That repository omits a license file; retain attribution to the original Atom
  palette. Panda maps terminal cyan to purple.
- Sonokai's own terminal implementation maps ANSI cyan to orange. Rose Pine's
  ANSI green slot is its blue Pine color, so Done uses this palette's Pine hue.
  Status labels and glyphs also identify their meaning.
- Shades of Purple specifies translucent terminal red `#ec3a37f5`; composite
  it over its terminal background `#1e1e3f` to obtain opaque `#e43937`.
- Zenbones is primarily a contrast and font-variation theme. Its terminal
  extras supply explicit dark/light backgrounds; the upstream theme is not
  inherently transparent. Keep Seshi's existing `mono` choice for inherited
  terminal colors.
- Seshi's gray slot repaints ANSI 8 (bright black), so the second pass takes
  each source's ANSI 8. Where that grey sits on the background (contrast
  under 1.5: GitHub Dark and Dark Dimmed, Gruvbox Baby, Nordic, Edge, Falcon,
  Mellifluous, Mellow, Aquarium), it uses the theme's comment colour instead.
  Tomorrow's iTerm files set ANSI 8 to black; its Xresources `#666666` is used.
- Falcon's blue `#635196` barely shows on Seshi's open row, so its accent is
  the theme's signature orange `#ff761a`.
- Oxocarbon's own terminal slots are not in ANSI order and it has no yellow;
  its slots are matched by meaning, with purple `#be95ff` (its warning colour)
  as yellow. Its greys are computed at load time and were reproduced from its
  code. Carbonfox's ANSI yellow is teal by the author's choice.
- GitHub, Gruvbox Material, Oceanic Next, Palenight, One Light, Edge, Code
  Dark, Embark, Mellow, Zephyr and Fluoromachine ship no terminal file; their
  slots come from the theme's own `terminal_color_*` settings or, failing that,
  its editor palette. Fluoromachine has no blue, so blue is its purple.
- Monokai Pro itself is commercial. These are the MIT-licensed Community
  Edition colors published by Monokai's own `monokai-pro` organization; it has
  no blue, so ANSI blue is its orange `#fc9867` as in that port.
- Mellifluous builds its colors from OkHSL at load time; the values were
  reproduced from its default dark config with its own conversion code.
- VS Code JSON themes (SynthWave '84, Horizon) set no ANSI white or bright
  black; their comment colors stand in, Horizon's composited over `#1c1e26`.
- Material uses its Darker style. Its own accent there is orange, so the
  accent is its blue `#82aaff`; its ANSI slots come from its terminal settings.
- Modus Vivendi's accent is its own accent setting `#00bcff`.

## Attribution and license notices

The color values are adapted from the projects above. Seshi's derived UI roles,
contrast adjustments, and status mappings are local adaptations. Retain these
notices with distributed copies containing the corresponding palettes.

The following upstream copyright notices apply to the MIT-licensed palettes:

- dracula/alacritty: Copyright (c) 2018 Dracula Theme
- nordtheme/alacritty: Copyright (c) 2016-present Sven Greb <development@svengreb.de> (https://www.svengreb.de)
- catppuccin/alacritty: Copyright (c) 2021 Catppuccin
- altercation/solarized: Copyright (c) 2011 Ethan Schoonover
- joshdick/onedark.vim: Copyright (c) 2015 Joshua Dick
- sonph/onehalf: Copyright (c) 2019 Son A. Pham <sp@sonpham.me>
- rebelot/kanagawa.nvim: Copyright (c) 2021 Tommaso Laurenzi
- sdras/night-owl-vscode-theme: Copyright (c) 2018 Sarah Drasner
- bluz71/vim-nightfly-colors: Copyright (c) 2020-present vim-nightfly-colors authors
- bluz71/vim-moonfly-colors: Copyright (c) 2017-present vim-moonfly-colors authors
- sainnhe/everforest: Copyright (c) 2019 sainnhe
- rose-pine/alacritty: Copyright (c) Rosé Pine
- wesbos/cobalt2-vscode: Copyright (c) 2018 Wes Bos, Roberto Achar
- nanotech/jellybeans.vim: Copyright (c) 2009-2019 NanoTech
- cocopon/iceberg.vim: Copyright (c) 2014 cocopon <cocopon@me.com>
- sainnhe/sonokai: Copyright (c) 2020 sainnhe
- ahmadawais/shades-of-purple-vscode: See the linked upstream license.
- siamak/atom-panda-syntax: Copyright (c) 2016 Siamak Mokhtari <hi@siamak.work>
- raphamorim/lucario: Copyright (c) 2014-present Raphael Amorim
- whatyouhide/vim-gotham: Copyright (c) 2014 Andrea Leopardi
- romainl/Apprentice: Copyright (c) 2016 Romain Lafourcade
- zenbones-theme/zenbones.nvim: Copyright (c) 2022 Michael Chris Lopez
- EdenEast/nightfox.nvim: Copyright (c) 2021 James Simpson
- projekt0n/github-nvim-theme: Copyright (c) 2021 projekt0n
- sainnhe/gruvbox-material: Copyright (c) 2020 sainnhe
- luisiacc/gruvbox-baby: Copyright (c) 2021 Luis Carlos Cordero
- chriskempson/tomorrow-theme: Copyright (C) 2011 Chris Kempson
- tinted-theming/schemes: Copyright (c) 2022 Tinted Theming
- drewtempelmeyer/palenight.vim: Copyright (c) 2015 Joshua Dick
- rakr/vim-one: Copyright (c) 2016 Ramzi Akremi
- monokai-pro/zed: Copyright (c) 2025 Monokai
- nyoom-engineering/oxocarbon.nvim: Copyright (c) 2022 Riccardo Mazzarini
- scottmckendry/cyberdream.nvim: Copyright (c) 2023 Scott McKendry
- AlexvZyl/nordic.nvim: Copyright (c) 2023 Alexander van Zyl
- rmehri01/onenord.nvim: Copyright (c) 2021 Ryan Mehri
- sainnhe/edge: Copyright (c) 2019 Sainnhepark
- Mofiqul/vscode.nvim: Copyright (c) 2021 Mofiqul Islam
- tomasiser/vim-code-dark: Copyright (c) 2017 Tomáš Iser
- savq/melange-nvim: Copyright (c) 2021 Sergio Alejandro Vargas
- fenetikm/falcon: Copyright (c) 2018 fenetikm
- embark-theme/vim: Copyright (c) 2020 embark-theme
- slugbyte/lackluster.nvim: Copyright (c) 2024 Duncan Marsh
- ribru17/bamboo.nvim: Copyright (c) 2021 Navarasu; Copyright (c) 2023 Riley Bruins
- ramojus/mellifluous.nvim: Copyright (c) 2024 Ramojus Lapinskas
- mellow-theme/mellow.nvim: Copyright (c) 2023 Rohit K Viswanath
- thesimonho/kanagawa-paper.nvim: Copyright (c) 2021 Tommaso Laurenzi; Copyright (c) 2024 Simon Ho
- nvimdev/zephyr-nvim: Copyright (c) 2020 Raphael
- oskarnurm/koda.nvim: Copyright (c) 2026 Karl Oskar Joosep Nurm
- FrenzyExists/aquarium-vim: Copyright (c) 2021 Detective Pikachu
- maxmx03/fluoromachine.nvim: Copyright (c) 2022 Dracula Theme
- robb0wen/synthwave-vscode: Copyright (c) 2019 Robb Owen
- jolaleye/horizon-theme-vscode: Copyright (c) 2018 Jonathan Olaleye

Gruvbox declares MIT/X11 in its README and identifies morhetz as its author in
its palette source; that revision contains no standalone copyright/license file.

The MIT license applies to the MIT notices above and Gruvbox:

```text
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

Tokyo Night by Folke Lemaitre and Ayu by Konstantin Pschera use Apache-2.0.
Their source palette values are adapted to Seshi UI roles as described above.
The shared Apache license text follows; see their linked original license files
for each project's application appendix. Oceanic Next by Mike Hartington ships
Neovim's Apache-2.0 license text without a project copyright line.

Miasma by xero is dedicated to the public domain under CC0-1.0.

Material (marko-cerovac/material.nvim, GPL-2.0), Modus Vivendi and Operandi
(protesilaos/modus-themes, part of GNU Emacs; Copyright (C) 2019-2026 Free
Software Foundation, Inc., GPL-3.0-or-later) and Bluloco (uloco/bluloco.nvim,
LGPL-3.0) use copyleft licenses. Seshi takes only their color values, which
are adapted to its UI roles as described above, not their code; see the linked
license files.

<details>
<summary>Apache License, Version 2.0</summary>

```text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS
```

</details>
