local plugins = require("lazy.core.config").plugins
local plugin_values = require("lazy.core.plugin").values

local function assert_disabled(name)
    local plugin = plugins[name]
    assert(not plugin or plugin.enabled == false, name .. " must be disabled")
end

local function assert_list(actual, expected, label)
    assert(
        vim.deep_equal(actual, expected),
        label
            .. " mismatch: expected "
            .. vim.inspect(expected)
            .. ", got "
            .. vim.inspect(actual)
    )
end

assert_disabled("none-ls.nvim")
assert_disabled("mason-null-ls.nvim")
assert_disabled("telescope.nvim")
assert_disabled("go.nvim")
assert_disabled("guihua.lua")
assert_disabled("text-case.nvim")

local astrolsp = plugin_values(plugins.astrolsp, "opts", false)
local expected_servers = {
    "cssls",
    "dockerls",
    "emmet_ls",
    "gopls",
    "html",
    "jsonls",
    "just",
    "lua_ls",
    "marksman",
    "sourcekit",
    "taplo",
    "vtsls",
    "yamlls",
}
local actual_servers = vim.deepcopy(astrolsp.servers)
table.sort(actual_servers)
assert_list(actual_servers, expected_servers, "Enabled LSP servers")

local mason_lspconfig =
    plugin_values(plugins["mason-lspconfig.nvim"], "opts", false)
assert(
    mason_lspconfig.automatic_enable == false,
    "Mason must not enable every installed server"
)

local conform = plugin_values(plugins["conform.nvim"], "opts", false)
assert_list(conform.formatters_by_ft.lua, { "stylua" }, "Lua formatters")
assert_list(conform.formatters_by_ft.rust, { "rustfmt" }, "Rust formatters")
assert_list(
    conform.formatters_by_ft.swift,
    { "swiftformat" },
    "Swift formatters"
)
assert_list(
    conform.formatters_by_ft.typescript,
    { "prettierd" },
    "TypeScript formatters"
)
assert(conform.formatters_by_ft.go[1] == "goimports", "Go must use goimports")

local lint = plugin_values(plugins["nvim-lint"], "opts", false)
assert_list(lint.linters_by_ft.swift, { "swiftlint" }, "Swift linters")
assert(
    type(lint.linters.swiftlint) == "table",
    "SwiftLint must be concrete for the community executable filter"
)

assert(
    vim.filetype.match({ filename = "test.tfvars" }) == "terraform-vars",
    "tfvars must use the terraform-vars file type"
)

vim.cmd.packadd("nvim.undotree")
assert(vim.fn.exists(":Undotree") == 2, "the Undotree command is missing")
assert(vim.g.loaded_netrwPlugin ~= nil, "netrw remote URI support is missing")
