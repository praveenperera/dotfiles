-- Set filetype for terraform files
vim.cmd("au BufRead,BufNewFile *.tfvars set filetype=terraform")

-- Set filetype for jinja
vim.cmd("au BufNewFile,BufRead *.j2,*.jinja set ft=jinja")

--  Config rustaceanvim
vim.g.rustaceanvim = require("config.rustaceanvim").config()

-- custom build command
local function setup_build_command()
    vim.api.nvim_buf_create_user_command(0, "BuildIOS", function()
        vim.notify(
            "Building ios...",
            vim.log.levels.INFO,
            { title = "Cove", timeout = 500 }
        )

        vim.fn.jobstart("just build-ios", {
            on_exit = function(_job_id, exit_code, _event_type)
                if exit_code ~= 0 then
                    vim.notify(
                        "Build failed with exit code: " .. exit_code,
                        vim.log.levels.ERROR
                    )
                    return
                end

                vim.notify(
                    "Build finished successfully",
                    vim.log.levels.INFO,
                    { timeout = 500 }
                )
            end,
        })
    end, {})

    vim.api.nvim_buf_set_keymap(
        0,
        "n",
        "<leader>xb",
        ":BuildIOS<CR>",
        { noremap = true, silent = true }
    )
end

local home = vim.fn.expand("$HOME")
local project_path = home .. "/code/bitcoinppl/cove/*"

vim.api.nvim_create_autocmd("BufEnter", {
    pattern = project_path,
    callback = function()
        setup_build_command()
    end,
})
