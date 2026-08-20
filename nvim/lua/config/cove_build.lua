local M = {}

local commands = {
    BuildIOS = {
        task = "build-ios",
        key = "<Leader>xb",
        desc = "Build Cove for iOS",
    },
    BuildIOSDevice = {
        task = "build-ios-debug-device",
        key = "<Leader>xB",
        desc = "Build Cove for an iOS device",
    },
}

local function run_task(bufnr, task)
    local root = vim.fs.root(bufnr, { "justfile", "Justfile", ".git" })
        or vim.fs.dirname(vim.api.nvim_buf_get_name(bufnr))

    vim.notify("Running `just " .. task .. "`", vim.log.levels.INFO, {
        title = "Cove",
        timeout = 500,
    })

    vim.system({ "just", task }, { cwd = root, text = true }, function(result)
        vim.schedule(function()
            if result.code == 0 then
                vim.notify(
                    "Build completed successfully",
                    vim.log.levels.INFO,
                    {
                        title = "Cove",
                        timeout = 500,
                    }
                )
                return
            end

            local details = vim.trim(result.stderr or "")
            if details ~= "" then
                details = "\n" .. details:sub(1, 2000)
            end

            vim.notify(
                "Build failed with exit code " .. result.code .. details,
                vim.log.levels.ERROR,
                { title = "Cove" }
            )
        end)
    end)
end

local function create_command(bufnr, name, definition)
    vim.api.nvim_buf_create_user_command(bufnr, name, function()
        run_task(bufnr, definition.task)
    end, { desc = definition.desc })
end

function M.setup_build_commands(bufnr)
    if not vim.api.nvim_buf_is_valid(bufnr) then
        return
    end

    local existing = vim.api.nvim_buf_get_commands(bufnr, {})
    for name, definition in pairs(commands) do
        if not existing[name] then
            create_command(bufnr, name, definition)
        end

        vim.keymap.set("n", definition.key, "<Cmd>" .. name .. "<CR>", {
            buffer = bufnr,
            desc = definition.desc,
            silent = true,
        })
    end
end

return M
