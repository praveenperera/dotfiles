;; -*- mode: emacs-lisp -*-
;; This file is loaded by Spacemacs at startup.
;; It must be stored in your home directory.

(defun dotspacemacs/layers ()
  "Configuration Layers declaration.
You should not put any user code in this function besides modifying the variable
values."
  (setq-default
   ;; Base distribution to use. This is a layer contained in the directory
   ;; `+distribution'. For now available distributions are `spacemacs-base'
   ;; or `spacemacs'. (default 'spacemacs)
   dotspacemacs-distribution 'spacemacs
   ;; List of additional paths where to look for configuration layers.
   ;; Paths must have a trailing slash (i.e. `~/.mycontribs/')
   dotspacemacs-configuration-layer-path '("~/.emacs.d/private/")
   ;; List of configuration layers to load. If it is the symbol `all' instead
   ;; of a list then all discovered layers will be installed.
   dotspacemacs-configuration-layers
   '(
     swift
     typescript
     sql
     rust
     octave
     yaml
     javascript
     ;; ----------------------------------------------------------------
     ;; Example of useful layers you may want to use right away.
     ;; Uncomment some layer names and press <SPC f e R> (Vim style) or
     ;; <M-m f e R> (Emacs style) to install them.
     ;; ----------------------------------------------------------------
     ;; auto-completion
     ;; better-defaults
     emacs-lisp
     markdown
     syntax-checking
     auto-completion
     racket
     erlang
     elixir
     docker
     colors
     elm
     ruby
     ruby-on-rails
     ocaml
     javascript
     react
     git
     osx
     html
     org
     (shell :variables
            shell-default-height 30
            shell-default-position 'bottom
            shell-default-shell 'term
            shell-default-term-shell "/bin/zsh")
     spell-checking
     version-control
     )
   ;; List of additional packages that will be installed without being
   ;; wrapped in a layer. If you need some configuration for these
   ;; packages, then consider creating a layer. You can also put the
   ;; configuration in `dotspacemacs/user-config'.
   dotspacemacs-additional-packages'(csv-mode
                                     (reason-mode
                                      :location (recipe
                                                 :repo "reasonml-editor/reason-mode"
                                                 :fetcher github
                                                 :files ("reason-mode.el" "refmt.el" "reason-interaction.el"))) 
                                     xah-css-mode
                                     drag-stuff
                                     rjsx-mode
                                     prettier-js
                                     add-node-modules-path
                                     string-inflection
                                     highlight-indent-guides)
   ;; A list of packages and/or extensions that will not be install and loaded.
   dotspacemacs-excluded-packages '()
   ;; If non-nil spacemacs will delete any orphan packages, i.e. packages that
   ;; are declared in a layer which is not a member of
   ;; the list `dotspacemacs-configuration-layers'. (default t)
   dotspacemacs-delete-orphan-packages t))

(defun dotspacemacs/init ()
  "Initialization function.
This function is called at the very startup of Spacemacs initialization
before layers configuration.
You should not put any user code in there besides modifying the variable
values."
  ;; This setq-default sexp is an exhaustive list of all the supported
  ;; spacemacs settings.
  (setq-default
   ;; If non nil ELPA repositories are contacted via HTTPS whenever it's
   ;; possible. Set it to nil if you have no way to use HTTPS in your
   ;; environment, otherwise it is strongly recommended to let it set to t.
   ;; This variable has no effect if Emacs is launched with the parameter
   ;; `--insecure' which forces the value of this variable to nil.
   ;; (default t)
   dotspacemacs-elpa-https t
   ;; Maximum allowed time in seconds to contact an ELPA repository.
   dotspacemacs-elpa-timeout 5
   ;; If non nil then spacemacs will check for updates at startup
   ;; when the current branch is not `develop'. (default t)
   dotspacemacs-check-for-update t
   ;; One of `vim', `emacs' or `hybrid'. Evil is always enabled but if the
   ;; variable is `emacs' then the `holy-mode' is enabled at startup. `hybrid'
   ;; uses emacs key bindings for vim's insert mode, but otherwise leaves evil
   ;; unchanged. (default 'vim)
   dotspacemacs-editing-style 'vim
   ;; If non nil output loading progress in `*Messages*' buffer. (default nil)
   dotspacemacs-verbose-loading nil
   ;; Specify the startup banner. Default value is `official', it displays
   ;; the official spacemacs logo. An integer value is the index of text
   ;; banner, `random' chooses a random text banner in `core/banners'
   ;; directory. A string value must be a path to an image format supported
   ;; by your Emacs build.
   ;; If the value is nil then no banner is displayed. (default 'official)
   dotspacemacs-startup-banner 'official
   ;; List of items to show in the startup buffer. If nil it is disabled.
   ;; Possible values are: `recents' `bookmarks' `projects'.
   ;; (default '(recents projects))
   dotspacemacs-startup-lists '(recents projects)
   ;; Number of recent files to show in the startup buffer. Ignored if
   ;; `dotspacemacs-startup-lists' doesn't include `recents'. (default 5)
   dotspacemacs-startup-recent-list-size 5
   ;; Default major mode of the scratch buffer (default `text-mode')
   dotspacemacs-scratch-mode 'text-mode
   ;; List of themes, the first of the list is loaded when spacemacs starts.
   ;; Press <SPC> T n to cycle to the next theme in the list (works great
   ;; with 2 themes variants, one dark and one light)
   dotspacemacs-themes '(monokai,
                         zenbun,
                         molokai
                         spacemacs-dark
                         spacemacs-light
                         solarized-light
                         solarized-dark
                         leuven)
   ;; If non nil the cursor color matches the state color in GUI Emacs.
   dotspacemacs-colorize-cursor-according-to-state t
   ;; Default font. `powerline-scale' allows to quickly tweak the mode-line
   ;; size to make separators look not too crappy.
   dotspacemacs-default-font '("Hack"
                               :size 13
                               :weight normal
                               :width normal
                               :powerline-scale 1.1)

   ;; The leader key
   dotspacemacs-leader-key "SPC"
   ;; The leader key accessible in `emacs state' and `insert state'

   ;; (default "M-m")
   dotspacemacs-emacs-leader-key "M-m"
   ;; Major mode leader key is a shortcut key which is the equivalent of
   ;; pressing `<leader> m`. Set it to `nil` to disable it. (default ",")
   dotspacemacs-major-mode-leader-key ","
   ;; Major mode leader key accessible in `emacs state' and `insert state'.
   ;; (default "C-M-m)
   dotspacemacs-major-mode-emacs-leader-key "C-M-m"
                                        ; ; These variables control whether separate commands are bound in the GUI to
   ;; the key pairs C-i, TAB and C-m, RET.
   ;; Setting it to a non-nil value, allows for separate commands under <C-i>
   ;; and TAB or <C-m> and RET.
   ;; In the terminal, these pairs are generally indistinguishable, so this only
   ;; works in the GUI. (default nil)
   dotspacemacs-distinguish-gui-tab nil
   ;; (Not implemented) dotspacemacs-distinguish-gui-ret nil
   ;; The command key used for Evil commands (ex-commands) and
   ;; Emacs commands (M-x).
   ;; By default the command key is `:' so ex-commands are executed like in Vim
   ;; with `:' and Emacs commands are executed with `<leader> :'.
   dotspacemacs-command-key "SPC"
   ;; If non nil `Y' is remapped to `y$'. (default t)
   dotspacemacs-remap-Y-to-y$ t
   ;; Name of the default layout (default "Default")
   dotspacemacs-default-layout-name "Default"
   ;; If non nil the default layout name is displayed in the mode-line.
   ;; (default nil)
   dotspacemacs-display-default-layout nil
   ;; If non nil then the last auto saved layouts are resume automatically upon
   ;; start. (default nil)
   dotspacemacs-auto-resume-layouts nil
   ;; Location where to auto-save files. Possible values are `original' to
   ;; auto-save the file in-place, `cache' to auto-save the file to another
   ;; file stored in the cache directory and `nil' to disable auto-saving.
   ;; (default 'cache)
   dotspacemacs-auto-save-file-location 'cache
   ;; Maximum number of rollback slots to keep in the cache. (default 5)
   dotspacemacs-max-rollback-slots 5
   ;; If non nil then `ido' replaces `helm' for some commands. For now only
   ;; `find-files' (SPC f f), `find-spacemacs-file' (SPC f e s), and
   ;; `find-contrib-file' (SPC f e c) are replaced. (default nil)
   dotspacemacs-use-ido nil
   ;; If non nil, `helm' will try to minimize the space it uses. (default nil)
   dotspacemacs-helm-resize nil
   ;; if non nil, the helm header is hidden when there is only one source.
   ;; (default nil)
   dotspacemacs-helm-no-header nil
   ;; define the position to display `helm', options are `bottom', `top',
   ;; `left', or `right'. (default 'bottom)
   dotspacemacs-helm-position 'bottom
   ;; If non nil the paste micro-state is enabled. When enabled pressing `p`
   ;; several times cycle between the kill ring content. (default nil)
   dotspacemacs-enable-paste-micro-state nil
   ;; Which-key delay in seconds. The which-key buffer is the popup listing
   ;; the commands bound to the current keystroke sequence. (default 0.4)
   dotspacemacs-which-key-delay 0.4
   ;; Which-key frame position. Possible values are `right', `bottom' and
   ;; `right-then-bottom'. right-then-bottom tries to display the frame to the
   ;; right; if there is insufficient space it displays it at the bottom.
   ;; (default 'bottom)
   dotspacemacs-which-key-position 'bottom
   ;; If non nil a progress bar is displayed when spacemacs is loading. This
   ;; may increase the boot time on some systems and emacs builds, set it to
   ;; nil to boost the loading time. (default t)
   dotspacemacs-loading-progress-bar t
   ;; If non nil the frame is fullscreen when Emacs starts up. (default nil)
   ;; (Emacs 24.4+ only)
   dotspacemacs-fullscreen-at-startup nil
   ;; If non nil `spacemacs/toggle-fullscreen' will not use native fullscreen.
   ;; Use to disable fullscreen animations in OSX. (default nil)
   dotspacemacs-fullscreen-use-non-native nil
   ;; If non nil the frame is maximized when Emacs starts up.
   ;; Takes effect only if `dotspacemacs-fullscreen-at-startup' is nil.
   ;; (default nil) (Emacs 24.4+ only)
   dotspacemacs-maximized-at-startup t
   ;; A value from the range (0..100), increasing opacity, which describes
   ;; the transparency level of a frame when it's active or selected.
   ;; Transparency can be toggled through `toggle-transparency'. (default 90)
   dotspacemacs-active-transparency 90
   ;; A value from the range (0..100), in increasing opacity, which describes
   ;; the transparency level of a frame when it's inactive or deselected.
   ;; Transparency can be toggled through `toggle-transparency'. (default 90)
   dotspacemacs-inactive-transparency 90
   ;; If non nil unicode symbols are displayed in the mode line. (default t)
   dotspacemacs-mode-line-unicode-symbols t
   ;; If non nil smooth scrolling (native-scrolling) is enabled. Smooth
   ;; scrolling overrides the default behavior of Emacs which recenters the
   ;; point when it reaches the top or bottom of the screen. (default t)
   dotspacemacs-smooth-scrolling t
   ;; If non nil line numbers are turned on in all `prog-mode' and `text-mode'
   ;; derivatives. If set to `relative', also turns on relative line numbers.
   ;; (default nil)
   dotspacemacs-line-numbers t
   ;; If non-nil smartparens-strict-mode will be enabled in programming modes.
   ;; (default nil)
   dotspacemacs-smartparens-strict-mode nil
   ;; Select a scope to highlight delimiters. Possible values are `any',
   ;; `current', `all' or `nil'. Default is `all' (highlight any scope and
   ;; emphasis the current one). (default 'all)
   dotspacemacs-highlight-delimiters 'all
   ;; If non nil advises quit functions to keep server open when quitting.
   ;; (default nil)
   dotspacemacs-persistent-server nil
   ;; List of search tool executable names. Spacemacs uses the first installed
   ;; tool of the list. Supported tools are `ag', `pt', `ack' and `grep'.
   ;; (default '("ag" "pt" "ack" "grep"))
   dotspacemacs-search-tools '("ag" "pt" "ack" "grep")
   ;; The default package repository used if no explicit repository has been
   ;; specified with an installed package.
   ;; Not used for now. (default nil)
   dotspacemacs-default-package-repository nil
   ;; Delete whitespace while saving buffer. Possible values are `all'
   ;; to aggressively delete empty line and long sequences of whitespace,
   ;; `trailing' to delete only the whitespace at end of lines, `changed'to
   ;; delete only whitespace for changed lines or `nil' to disable cleanup.
   ;; (default nil)
   dotspacemacs-whitespace-cleanup nil
   ))

(defun dotspacemacs/user-init ()
  "Initialization function for user code.
It is called immediately after `dotspacemacs/init', before layer configuration
executes.
 This function is mostly useful for variables that need to be set
before packages are loaded. If you are unsure, you should try in setting them in
`dotspacemacs/user-config' first."


  ;; prettier settings
  (setq prettier-js-args '(
                           "--trailing-comma" "es5"
                        ))

  ;; prettier hooks
  (add-hook 'js2-mode-hook 'prettier-js-mode)
  (add-hook 'react-mode-hook 'prettier-js-mode)

  ;; rustfmt on save
  (setq rust-format-on-save t)

  ;; node-module-path
  (eval-after-load 'js2-mode
    '(add-hook 'js2-mode-hook #'add-node-modules-path))


  (eval-after-load 'react-mode
    '(add-hook 'react-mode-hook #'add-node-modules-path))



  ;; Elixir format

  ;; Work around to make .formatter work with formatter dpes
  ;; Runs mix format from mix project root
  (defun set-default-directory-to-mix-project-root (original-fun &rest args)
    (if-let* ((mix-project-root (and (projectile-project-p)
                                     (projectile-locate-dominating-file buffer-file-name
                                                                        ".formatter.exs"))))
        (let ((default-directory mix-project-root))
          (apply original-fun args))
      (apply original-fun args)))

  (advice-add 'elixir-format :around #'set-default-directory-to-mix-project-root)

  ;; Create a buffer-local hook to run elixir-format on save, only when we enable elixir-mode.
  (add-hook 'elixir-mode-hook
            (lambda () (add-hook 'before-save-hook 'elixir-format nil t)))

  (add-hook 'elixir-format-hook (lambda ()
                                  (if (projectile-project-p)
                                      (setq elixir-format-arguments
                                            (list "--dot-formatter"
                                                  (concat (locate-dominating-file buffer-file-name ".formatter.exs") ".formatter.exs")))
                                    (setq elixir-format-arguments nil))))

  ;; disable creation of .# files
  (setq create-lockfiles nil)

  (setq-default
   ;; js2-mods
   js2-basic-offset 2
   ;; web-mode
   css-indent-offset 2
   web-mode-markup-indent-offset 2
   web-mode-css-indent-offset 2
   web-mode-code-indent-offset 2
   web-mode-attr-indent-offset 2)

  (add-to-list 'auto-mode-alist '("\\.erb\\'" . web-mode))
  (setq web-mode-content-types-alist
        '(("jsx" . "\\.js[x]?\\'")))


  ;;turn off elm indent mode
  (add-hook 'elm-mode-hook #'turn-off-elm-indent)

  ;; Use emmet mode in other modes (React and Haml)
  (add-hook 'react-mode-hook 'emmet-mode)
  (add-hook 'haml-mode-hook 'emmet-mode)

  (setq web-mode-markup-indent-offset 2
        web-mode-css-indent-offset 2
        web-mode-code-indent-offset 2)
  (setq js-indent-level 2)
  (setq css-indent-offset 2)

  ;; REASONML SETTINGS
  ;; enable refmt on save
  (add-hook 'reason-mode-hook (lambda ()
                                (add-hook 'before-save-hook #'refmt-before-save)))


  ;; enable emmet on reason mode
  (add-hook 'reason-mode-hook 'emmet-mode)
  ;; / REASONML SETTINGS

  (with-eval-after-load 'web-mode
    (add-to-list 'web-mode-indentation-params '("lineup-args" . nil))
    (add-to-list 'web-mode-indentation-params '("lineup-concats" . nil))
    (add-to-list 'web-mode-indentation-params '("lineup-calls" . nil)))


  (setq-default dotspacemacs-configuration-layers
                '((auto-completion :variables
                                   auto-completion-enable-snippets-in-popup t)))

  (defun web-mode-customization-hook ()
    "Hooks for Web mode."
    (setq web-mode-markup-indent-offset 2)
    (setq web-mode-scss-indent-offset 2)
    (setq web-mode-css-indent-offset 2)
    (setq web-mode-code-indent-offset 2)
    )

  (defun emmet-mode-customization-hook ()
    "Hooks for Emmet mode."
    (setq emmet-indentation 2 )
    (setq emmet-indent-after-insert nil)
    )

 (add-hook 'web-mode-hook  'web-mode-customization-hook)
 (add-hook 'emmet-mode-hook 'emmet-mode-customization-hook)
 (add-hook 'projectile-after-switch-project-hook 'mjs/setup-local-eslint)

 ;; Hack fix for highlight indent guide
 (defun my-unfontify-function (beg end)
   (remove-list-of-text-properties beg end '(display)))

 (defun my-register-unfontify ()
   (setq font-lock-unfontify-region-function 'my-unfontify-function))

 (add-hook 'web-mode-hook 'my-register-unfontify t)


 (defun setup-local-eslint  ()
   "If ESLint found in node_modules directory - use that for flycheck.
        Intended for use in PROJECTILE-AFTER-SWITCH-PROJECT-HOOK."
   (interactive)
   (let ((local-eslint  (expand-file-name "./node_modules/.bin/eslint")))
     (setq flycheck-javascript-eslint-executable
           (and (file-exists-p local-eslint) local-eslint))))
 (global-auto-revert-mode t)
 )


(defun dotspacemacs/user-config ()
  "Configuration function for user code.
This function is called at the very end of Spacemacs initialization after
layers configuration.
This is the place where most of your configurations should be done. Unless it is
explicitly specified that a variable should be set before a package is loaded,
you should place you code here."
  (setq powerline-default-separator 'arrow)
  (spaceline-compile)

  ;; Enable global linum in relative mode
  ;; Make linums relative by default
  (global-linum-mode)
  (linum-relative-toggle)

  ;; Fix helm bug
  (require 'helm-bookmark)

  ;; Allow moving lines with OPT-<ARROW>
  (global-set-key (kbd "M-<up>")   #'drag-stuff-up)
  (global-set-key (kbd "M-<down>") #'drag-stuff-down)

  ;; enable global multi cursor mode
  (global-evil-mc-mode  1)

  ;; Indent highlight settings
  (spacemacs/set-leader-keys "t h i" 'highlight-indent-guides-mode)
  (setq highlight-indent-guides-method 'character)
  (add-hook 'prog-mode-hook 'highlight-indent-guides-mode)

  ;;elm format variable
  (setq elm-format-command "elm-format-0.18" ) 

  ;; ;; Search using ripgrep (rg)
  (custom-set-variables
   '(helm-ag-base-command "rg --no-heading"))

  ;; CUSTOM KEYBINDINGS
  ;; note can use timing using key-chord-define
  (spacemacs/set-leader-keys "[" 'multi-term)
  (spacemacs/set-leader-keys "je" 'alchemist-goto-definition-at-point)
  ;; (spacemacs/set-leader-keys "]" 'column-highlight-mode)
  ;; (define-key evil-normal-state-map (kbd "']") 'column-highlight-mode)


  ;; CUSTOM KEYBINDING FOR MAJORMODE
  ;; ELIXIR
  (spacemacs/set-leader-keys-for-major-mode 'elixir-mode "gg" 'alchemist-goto-definition-at-point)

  (defun evil-paste-after-from-0 ()
    (interactive)
    (let ((evil-this-register ?0))
      (call-interactively 'evil-paste-after)))

  (define-key evil-visual-state-map "p" 'evil-paste-after-from-0)

  (define-key evil-normal-state-map "gS" 'string-inflection-lower-camelcase)
  (define-key evil-normal-state-map "gs" 'string-inflection-underscore)

  (with-eval-after-load 'company
    (add-to-list 'company-backends 'company-elm))

  ;; Show 80-column marker
  (define-globalized-minor-mode global-fci-mode fci-mode (lambda () (fci-mode 1)))
  (global-fci-mode 1)
  (setq fci-rule-color "gray22")

  (add-to-list 'auto-mode-alist '("\\.jsx?$" . react-mode))
  (add-to-list 'spacemacs-indent-sensitive-modes 'elixir-mode)
  )
;; Do not write anything past this comment. This is where Emacs will
;; auto-generate custom variable definitions.

(custom-set-variables
 ;; custom-set-variables was added by Custom.
 ;; If you edit it by hand, you could mess it up, so be careful.
 ;; Your init file should contain only one such instance.
 ;; If there is more than one, they won't work right.
 '(ansi-color-faces-vector
   [default bold shadow italic underline bold bold-italic bold])
 '(compilation-message-face (quote default))
 '(custom-safe-themes
   (quote
    ("28ec8ccf6190f6a73812df9bc91df54ce1d6132f18b4c8fcc85d45298569eb53" "66132890ee1f884b4f8e901f0c61c5ed078809626a547dbefbb201f900d03fd8" "a1289424bbc0e9f9877aa2c9a03c7dfd2835ea51d8781a0bf9e2415101f70a7e" "6254372d3ffe543979f21c4a4179cd819b808e5dd0f1787e2a2a647f5759c1d1" "d8f76414f8f2dcb045a37eb155bfaa2e1d17b6573ed43fb1d18b936febc7bbc2" default)))
 '(ediff-split-window-function (quote split-window-horizontally) t)
 '(ediff-window-setup-function (quote ediff-setup-windows-plain) t)
 '(elm-format-on-save t)
 '(elm-sort-imports-on-save t)
 '(elm-tags-on-save t)
 '(evil-want-Y-yank-to-eol t)
 '(flycheck-rubocop-lint-only t)
 '(helm-ag-base-command "rg --no-heading")
 '(highlight-changes-colors (quote ("#FD5FF0" "#AE81FF")))
 '(highlight-tail-colors
   (quote
    (("#3E3D31" . 0)
     ("#67930F" . 20)
     ("#349B8D" . 30)
     ("#21889B" . 50)
     ("#968B26" . 60)
     ("#A45E0A" . 70)
     ("#A41F99" . 85)
     ("#3E3D31" . 100))))
 '(hl-sexp-background-color "#1c1f26")
 '(magit-commit-arguments (quote ("--gpg-sign=8A7A38239BD46ACF")))
 '(magit-diff-use-overlays nil)
 '(mixfmt-elixir "/usr/local/bin/elixir")
 '(mixfmt-mix "/usr/local/bin/mix")
 '(nrepl-message-colors
   (quote
    ("#CC9393" "#DFAF8F" "#F0DFAF" "#7F9F7F" "#BFEBBF" "#93E0E3" "#94BFF3" "#DC8CC3")))
 '(package-selected-packages
   (quote
    (docker-tramp tablist transient lv reformatter racket-mode faceup treepy graphql ripgrep drag-stuff dockerfile-mode docker add-node-modules-path swift-mode evil-string-inflection string-inflection zenity-color-picker pandoc-mode ox-pandoc ht spotify helm-spotify-plus multi org-mime ghub let-alist flymake-elixir reason-mode tide typescript-mode utop tuareg caml ocp-indent merlin elixir-format org-category-capture sql-indent prettier-js toml-mode racer flycheck-rust seq cargo rust-mode highlight-indent-guides uuidgen rjsx-mode livid-mode pos-tip evil-visual-mark-mode evil-ediff goto-chg f diminish darkokai-theme web-completion-data dash-functional bind-key pkg-info epl popup winum solarized-theme madhat2r-theme fuzzy flycheck-credo avy link-hint xah-css-mode packed simple-httpd flymake-jslint flycheck-mix eshell-z crosshairs evil-terminal-cursor-changer column-marker relative-line-numbers nlinum autothemer tern atom-one-dark-theme-theme atom-dark-theme skewer-mode org-download osx-dictionary company org-projectile pcache flyspell-correct-helm auto-complete eyebrowse git-link color-identifiers-mode inf-ruby yaml-mode pug-mode ob-elixir org minitest hide-comnt dumb-jump column-enforce-mode flyspell-correct dash yasnippet async evil-unimpaired elixir-mode undo-tree helm helm-core s zonokai-theme zenburn-theme zen-and-art-theme xterm-color ws-butler window-numbering web-mode web-beautify volatile-highlights vi-tilde-fringe underwater-theme ujelly-theme twilight-theme twilight-bright-theme twilight-anti-bright-theme tronesque-theme toxi-theme toc-org tao-theme tangotango-theme tango-plus-theme tango-2-theme tagedit sunny-day-theme sublime-themes subatomic256-theme subatomic-theme stekene-theme spacemacs-theme spaceline powerline spacegray-theme soothe-theme soft-stone-theme soft-morning-theme soft-charcoal-theme smyx-theme smooth-scrolling smeargle slim-mode shell-pop seti-theme scss-mode sass-mode rvm ruby-tools ruby-test-mode ruby-end rubocop rspec-mode robe reverse-theme reveal-in-osx-finder restart-emacs rbenv rainbow-mode rainbow-identifiers rainbow-delimiters railscasts-theme purple-haze-theme projectile-rails rake inflections professional-theme popwin planet-theme phoenix-dark-pink-theme phoenix-dark-mono-theme persp-mode pcre2el pbcopy pastels-on-dark-theme paradox hydra spinner page-break-lines osx-trash orgit organic-green-theme org-repo-todo org-present org-pomodoro alert log4e gntp org-plus-contrib org-bullets open-junk-file omtose-phellack-theme oldlace-theme occidental-theme obsidian-theme noctilux-theme niflheim-theme neotree naquadah-theme mustang-theme multi-term move-text monokai-theme monochrome-theme molokai-theme moe-theme mmm-mode minimal-theme material-theme markdown-toc markdown-mode majapahit-theme magit-gitflow macrostep lush-theme lorem-ipsum linum-relative light-soap-theme leuven-theme less-css-mode launchctl json-mode json-snatcher json-reformat js2-refactor multiple-cursors js2-mode js-doc jbeans-theme jazz-theme jade-mode ir-black-theme inkpot-theme info+ indent-guide ido-vertical-mode hungry-delete htmlize hl-todo highlight-parentheses highlight-numbers parent-mode highlight-indentation heroku-theme hemisu-theme help-fns+ helm-themes helm-swoop helm-projectile helm-mode-manager helm-make projectile helm-gitignore request helm-flyspell helm-flx helm-descbinds helm-css-scss helm-company helm-c-yasnippet helm-ag hc-zenburn-theme haml-mode gruvbox-theme gruber-darker-theme grandshell-theme gotham-theme google-translate golden-ratio gnuplot gitignore-mode gitconfig-mode gitattributes-mode git-timemachine git-messenger git-gutter-fringe+ git-gutter-fringe fringe-helper git-gutter+ git-gutter gh-md gandalf-theme flycheck-pos-tip flycheck-elm flycheck flx-ido flx flatui-theme flatland-theme firebelly-theme fill-column-indicator feature-mode farmhouse-theme fancy-battery expand-region exec-path-from-shell evil-visualstar evil-tutor evil-surround evil-search-highlight-persist evil-numbers evil-nerd-commenter evil-mc evil-matchit evil-magit magit magit-popup git-commit with-editor evil-lisp-state smartparens evil-indent-plus evil-iedit-state iedit evil-exchange evil-escape evil-args evil-anzu anzu eval-sexp-fu highlight espresso-theme eshell-prompt-extras csv-mode company-statistics adaptive-wrap which-key use-package quelpa evil esh-help erlang emmet-mode elm-mode elisp-slime-nav dracula-theme django-theme diff-hl define-word darktooth-theme darkmine-theme darkburn-theme dakrone-theme cyberpunk-theme company-web company-tern company-quickhelp colorsarenice-theme color-theme-sanityinc-tomorrow color-theme-sanityinc-solarized col-highlight coffee-mode clues-theme clean-aindent-mode chruby cherry-blossom-theme busybee-theme bundler buffer-move bubbleberry-theme bracketed-paste birds-of-paradise-plus-theme bind-map badwolf-theme auto-yasnippet auto-highlight-symbol auto-dictionary auto-compile atom-one-dark-theme apropospriate-theme anti-zenburn-theme ample-zen-theme ample-theme alect-themes alchemist aggressive-indent afternoon-theme ace-window ace-link ace-jump-helm-line ac-ispell)))
 '(pdf-view-midnight-colors (quote ("#DCDCCC" . "#383838")))
 '(pos-tip-background-color "#A6E22E")
 '(pos-tip-foreground-color "#272822")
 '(vc-annotate-background "#2B2B2B")
 '(vc-annotate-color-map
   (quote
    ((20 . "#BC8383")
     (40 . "#CC9393")
     (60 . "#DFAF8F")
     (80 . "#D0BF8F")
     (100 . "#E0CF9F")
     (120 . "#F0DFAF")
     (140 . "#5F7F5F")
     (160 . "#7F9F7F")
     (180 . "#8FB28F")
     (200 . "#9FC59F")
     (220 . "#AFD8AF")
     (240 . "#BFEBBF")
     (260 . "#93E0E3")
     (280 . "#6CA0A3")
     (300 . "#7CB8BB")
     (320 . "#8CD0D3")
     (340 . "#94BFF3")
     (360 . "#DC8CC3"))))
 '(vc-annotate-very-old-color "#DC8CC3")
 '(weechat-color-list
   (unspecified "#272822" "#3E3D31" "#A20C41" "#F92672" "#67930F" "#A6E22E" "#968B26" "#E6DB74" "#21889B" "#66D9EF" "#A41F99" "#FD5FF0" "#349B8D" "#A1EFE4" "#F8F8F2" "#F8F8F0")))
(custom-set-faces
 ;; custom-set-faces was added by Custom.
 ;; If you edit it by hand, you could mess it up, so be careful.
 ;; Your init file should contain only one such instance.
 ;; If there is more than one, they won't work right.
 '(default ((t (:foreground "#ABB2BF" :background "#282C34"))))
 '(ediff-current-diff-C ((t (:background "goldenrod"))))
 '(ediff-even-diff-A ((t (:background "gray40"))))
 '(ediff-even-diff-Ancestor ((t (:background "gray30"))))
 '(ediff-even-diff-B ((t (:background "gray40"))))
 '(ediff-even-diff-C ((t (:background "grey40"))))
 '(ediff-odd-diff-A ((t (:background "gray40"))))
 '(ediff-odd-diff-Ancestor ((t (:background "gray25"))))
 '(ediff-odd-diff-B ((t (:background "gray40"))))
 '(ediff-odd-diff-C ((t (:background "gray30"))))
 '(web-mode-block-delimiter-face ((t (:foreground "tomato1"))))
 '(web-mode-html-attr-name-face ((t (:foreground "Tan")))))
