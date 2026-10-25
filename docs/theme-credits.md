# Theme sources

Research checked on 2026-10-14 against the creators' source repositories. The
catalog covers all 25 requested families with 42 palettes. Before this expansion,
Maitty already included Dracula, Gruvbox Dark, Nord, Tokyo Night Night, and
Catppuccin Mocha/Latte. These are six palettes across five families.

A second pass on 2026-10-25 added the families from Cyberdream on in the table,
each at its creator's revision. Vividchalk (Vim license), Blackboard and Wombat
(no license on the original or any creator-owned port) are left out.

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
| Cyberdream | Added | Default / Light / Muted | [scottmckendry/cyberdream.nvim](https://github.com/scottmckendry/cyberdream.nvim/blob/81b3e39ceb87db2987255123af3212beeb75bf6d/lua/cyberdream/colors.lua) | [MIT](https://github.com/scottmckendry/cyberdream.nvim/blob/81b3e39ceb87db2987255123af3212beeb75bf6d/LICENSE) |
| Solarized Osaka | Added | Dark / Light | [craftzdog/solarized-osaka.nvim](https://github.com/craftzdog/solarized-osaka.nvim/blob/a21176efde15fab21fabda0bf2ba123ed6d7332d/extras/alacritty/solarized_osaka_dark.toml) | [Apache-2.0](https://github.com/craftzdog/solarized-osaka.nvim/blob/a21176efde15fab21fabda0bf2ba123ed6d7332d/LICENSE) |
| Melange | Added | Dark / Light | [savq/melange-nvim](https://github.com/savq/melange-nvim/blob/ce42f6b629beeaa00591ba73a77d3eeac4cf28ce/term/alacritty/melange_dark.toml) | [MIT](https://github.com/savq/melange-nvim/blob/ce42f6b629beeaa00591ba73a77d3eeac4cf28ce/LICENSE) |
| Bamboo | Added | Vulgaris / Multiplex / Light | [ribru17/bamboo.nvim](https://github.com/ribru17/bamboo.nvim/blob/1309bc88bffcf1bedc3e84e7fa9004de93da774a/extras/alacritty/bamboo.toml) | [MIT](https://github.com/ribru17/bamboo.nvim/blob/1309bc88bffcf1bedc3e84e7fa9004de93da774a/LICENSE) |
| Gruber Darker | Added | Standard | [blazkowolf/gruber-darker.nvim](https://github.com/blazkowolf/gruber-darker.nvim/blob/35cb97959ef01f7193c94c404c13ddb3d4346654/extras/alacritty/gruber-darker.toml) | [MIT](https://github.com/blazkowolf/gruber-darker.nvim/blob/35cb97959ef01f7193c94c404c13ddb3d4346654/LICENSE) |
| Vague | Added | Standard | [vague-theme/vague.nvim](https://github.com/vague-theme/vague.nvim/blob/73f8aaa866bf3d14dd0b0ab9e48c11794c1cb7ab/lua/vague/terminal.lua) | [MIT](https://github.com/vague-theme/vague.nvim/blob/73f8aaa866bf3d14dd0b0ab9e48c11794c1cb7ab/LICENSE) |
| Tender | Added | Standard | [jacoborus/tender.vim](https://github.com/jacoborus/tender.vim/blob/b66dc330aff998dff9d44459d84bc1c5f4304fcc/colors/tender.vim) | [MIT](https://github.com/jacoborus/tender.vim/blob/b66dc330aff998dff9d44459d84bc1c5f4304fcc/LICENSE) |
| Ashen | Added | Default | [ficd0/ashen.nvim](https://github.com/ficd0/ashen.nvim/blob/bfb04bd50b69d863469b2deb9fb361cf0d945ba7/lua/ashen/variants/default/colors.lua) | [MIT](https://github.com/ficd0/ashen.nvim/blob/bfb04bd50b69d863469b2deb9fb361cf0d945ba7/LICENSE) |
| Brogrammer | Added | Standard | [kenwheeler/brogrammer-syntax](https://github.com/kenwheeler/brogrammer-syntax/blob/8e3c66d1fcf5f3a3918344e63f6f0fa287369aab/styles/colors.less) | [MIT](https://github.com/kenwheeler/brogrammer-syntax/blob/8e3c66d1fcf5f3a3918344e63f6f0fa287369aab/LICENSE.md) |
| Palenight | Added | Standard | [drewtempelmeyer/palenight.vim](https://github.com/drewtempelmeyer/palenight.vim/blob/9e9b102c54b830b72f51917b6b86c3b3b02dcea2/autoload/palenight.vim) | [MIT](https://github.com/drewtempelmeyer/palenight.vim/blob/9e9b102c54b830b72f51917b6b86c3b3b02dcea2/LICENSE) |
| Neovim default | Added | Dark / Light | [neovim/neovim](https://github.com/neovim/neovim/blob/80ae7e336f97e169bb3d3e7172998a65336472a1/src/nvim/highlight_group.c) | [Apache-2.0](https://github.com/neovim/neovim/blob/80ae7e336f97e169bb3d3e7172998a65336472a1/LICENSE.txt) |
| Nightfox | Added | Nightfox / Dayfox / Dawnfox / Duskfox / Nordfox / Terafox / Carbonfox | [EdenEast/nightfox.nvim](https://github.com/EdenEast/nightfox.nvim/blob/4dacd3f0185a2227bdf3b6c0975a8f0bf87cac9a/extra/nightfox/alacritty.toml) | [MIT](https://github.com/EdenEast/nightfox.nvim/blob/4dacd3f0185a2227bdf3b6c0975a8f0bf87cac9a/LICENSE) |
| Gruvbox Material | Added | Dark / Light, medium background | [sainnhe/gruvbox-material](https://github.com/sainnhe/gruvbox-material/blob/11d779b26a9ab2b3db8c22c6ac9fb6e8ed4fea79/autoload/gruvbox_material.vim) | [MIT](https://github.com/sainnhe/gruvbox-material/blob/11d779b26a9ab2b3db8c22c6ac9fb6e8ed4fea79/LICENSE) |
| Doom One | Added | Dark / Light | [NTBBloodbath/doom-one.nvim](https://github.com/NTBBloodbath/doom-one.nvim/blob/960d8bf49904001b7cf37fdc34ca18dffb21b024/extras/alacritty-dark.toml) | [MIT](https://github.com/NTBBloodbath/doom-one.nvim/blob/960d8bf49904001b7cf37fdc34ca18dffb21b024/LICENSE) |
| Mellow | Added | Dark / Light | [mellow-theme/mellow.nvim](https://github.com/mellow-theme/mellow.nvim/blob/6273b7f0ddf6ac2b75021b3bc0bd8b3305cfeef1/lua/mellow/colors.lua) | [MIT](https://github.com/mellow-theme/mellow.nvim/blob/6273b7f0ddf6ac2b75021b3bc0bd8b3305cfeef1/LICENSE) |
| Koda | Added | Dark / Light / Moss / Glade | [oskarnurm/koda.nvim](https://github.com/oskarnurm/koda.nvim/blob/25a5c98ca2f6bb0b69f5e2654a78f0d7c7e0721d/extras/ghostty/koda-dark.conf) | [MIT](https://github.com/oskarnurm/koda.nvim/blob/25a5c98ca2f6bb0b69f5e2654a78f0d7c7e0721d/LICENSE) |
| Code 2026 | Added | Dark / Light | [code-2026-theme/ghostty](https://github.com/code-2026-theme/ghostty/blob/60da4559b02d0f0f2fced8ea4c696d29eff0190c/themes/code-2026.conf) | [MIT](https://github.com/code-2026-theme/ghostty/blob/60da4559b02d0f0f2fced8ea4c696d29eff0190c/LICENSE) |

## Mapping to Maitty

These are terminal palettes adapted to Maitty's UI roles, rather than editor
syntax themes reproduced verbatim. Keep the original background, foreground,
and ANSI accent identities; derive surfaces, borders, muted labels, and status
roles for Maitty's contrast and semantic checks. Upstream terminal bright-black
colors are frequently unsuitable for small UI labels. A selection background
may assume dark selected text, so it is not automatically a valid menu fill.

- Catppuccin supplies four distinct flavors; Latte's source foreground is
  `#4c4f69`. The older Maitty Latte foreground was not the canonical Latte text.
- Tokyo Night includes the official Moon terminal variant as well as Night,
  Storm, and Day. Its Day foreground needs the UI's strongest-text contrast
  adjustment on derived surfaces.
- Gruvbox uses the source's bright accents in dark mode and faded accents in
  light mode; both use its default medium background contrast.
- Solarized uses Ethan Schoonover's canonical sRGB base and accent colors.
  Its precise palette does not imply that every accent passes Maitty's UI
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
  inherently transparent. Keep Maitty's existing `mono` choice for inherited
  terminal colors.

From the 2026-10-25 pass on, each new palette maps the same way: the source's
background and foreground, its comment color as the dim role, its blue as the
accent, and its ANSI red, green, yellow, blue, magenta and cyan. The gray slot
(what programs' bright black becomes) takes the source's ANSI 8, or its comment
color where ANSI 8 is missing or sits on the background (Solarized Osaka,
Tender, Palenight, Brogrammer, Neovim).

- Cyberdream's Muted variant is its own `variant = "muted"` palette.
- Solarized Osaka's Vivid style is computed at runtime from the default
  palette, so it has no fixed values to take. Its comment color is the
  source's `base01`, `hsl(194, 14, 40)`, as `#576d74` (light: `base1`,
  `#adb7b7`), with the source's truncating conversion.
- Gruber Darker's comments are brown, which would read as the needs-you amber,
  so its dim role is its `bg+4` grey `#52494e`.
- Ashen's terminal green and yellow slots are its two oranges, so Done shows
  orange in Ashen; its status labels and glyphs still say which state it is.
- Brogrammer's colors come from Ken Wheeler's MIT-licensed Atom port of his
  Sublime theme; the Sublime repository has no license. It defines no terminal
  palette, so the slots take its named colors.
- Neovim's default colorscheme sets no terminal colors. Its dark variant takes
  the `NvimLight*` colors it draws text with, the light one the `NvimDark*`.
- Nightfox's comment colors come from each variant's
  `lua/nightfox/palette/<variant>.lua`; Carbonfox computes its comment as
  `bg:blend(fg, 0.4)`, `#6e6f70`. Carbonfox's terminal yellow slot is teal, so
  its needs-you amber is Maitty's own.
- Gruvbox Material takes the medium background and the default material
  foreground; its terminal mapping is in `colors/gruvbox-material.vim`.
- Doom One's light palette is `extras/alacritty-light.toml`; both comment
  colors come from `lua/doom-one/colors.lua`.
- Mellow's light variant is `vim.g.mellow_variant = "light"`. Its terminal cyan
  slot is rose in both variants.
- Koda's Dark and Light take the creator's Ghostty files. Moss and Glade ship
  no terminal palette, so their slots take the named colors in
  `lua/koda/palette/<variant>.lua`. Koda Dark's terminal yellow is white and
  its blue grey: the theme is near-monochrome by design.
- Code 2026 ports VS Code's 2026 themes. The terminal palettes are the
  creator's Ghostty files (`themes/code-2026.conf`, `themes/light-2026.conf`);
  the comment colors come from
  [code-2026-theme/nvim](https://github.com/code-2026-theme/nvim/blob/ea0b1fd2751560f587aa3c070e30b7fdf1b89d27/lua/code-2026/palette.lua)
  ([MIT](https://github.com/code-2026-theme/nvim/blob/ea0b1fd2751560f587aa3c070e30b7fdf1b89d27/LICENSE)).

## Attribution and license notices

The color values are adapted from the projects above. Maitty's derived UI roles,
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
- scottmckendry/cyberdream.nvim: Copyright (c) 2023 Scott McKendry
- savq/melange-nvim: Copyright (c) 2021 Sergio Alejandro Vargas
- ribru17/bamboo.nvim: Original Copyright (c) 2021 Navarasu; Copyright (c) 2023 Riley Bruins
- blazkowolf/gruber-darker.nvim: Copyright (c) 2023 blazkowolf
- vague-theme/vague.nvim: Copyright (c) 2024 Alberto Hernandez
- jacoborus/tender.vim: Copyright (c) 2020 Jacobo Tabernero
- ficd0/ashen.nvim: Copyright © 2025 Daniel Fichtinger, GitHub contributors
- kenwheeler/brogrammer-syntax: MIT per its `package.json`; its license file
  keeps the template line `Copyright (c) 2014 <Your name here>`.
- drewtempelmeyer/palenight.vim: Copyright (c) 2015 Joshua Dick
- EdenEast/nightfox.nvim: Copyright (c) 2021 James Simpson
- sainnhe/gruvbox-material: Copyright (c) 2020 sainnhe
- NTBBloodbath/doom-one.nvim: Copyright (c) 2021 NTBBloodbath
- mellow-theme/mellow.nvim: Copyright (c) 2023 Rohit K Viswanath
- oskarnurm/koda.nvim: Copyright (c) 2026 Karl Oskar Joosep Nurm
- code-2026-theme/ghostty and code-2026-theme/nvim: Copyright (c) 2026 code-2026-theme

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

Tokyo Night by Folke Lemaitre, Ayu by Konstantin Pschera, Solarized Osaka
(Copyright 2024 Takuya Matsuyama) and Neovim's default colorscheme (Copyright
Neovim contributors; its colors are Neovim's own Apache-2.0 contribution, not
Vim-licensed code) use Apache-2.0.
Their source palette values are adapted to Maitty UI roles as described above.
The shared Apache license text follows; see their linked original license files
for each project's application appendix.

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
