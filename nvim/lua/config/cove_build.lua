local M = {}

M.setup_build_commands = function()
    local function notify_build_start(message)
        vim.notify(
            message,
            vim.log.levels.INFO,
            { title = "Cove", timeout = 500 }
        )
    end

    local function create_build_command(command)
        return function()
            notify_build_start("Running `just " .. command .. "` ...")

            vim.fn.jobstart("just " .. command, {
                on_exit = function(_job_id, exit_code, _event_type)
                    if exit_code ~= 0 then
                        vim.notify(
                            "Build failed with exit code: " .. exit_code,
                            vim.log.levels.ERROR
                        )
                        return
                    end

                    vim.notify(
                        "Build completed successfully",
                        vim.log.levels.INFO,
                        { timeout = 500 }
                    )
                end,
            })
        end
    end

    vim.api.nvim_buf_create_user_command(
        0,
        "BuildIOS",
        create_build_command("build-ios"),
        {}
    )

    vim.api.nvim_buf_create_user_command(
        0,
        "BuildIOSDevice",
        create_build_command("build-ios-debug-device"),
        {}
    )

    local function set_keymap(key, command)
        vim.api.nvim_buf_set_keymap(
            0,
            "n",
            key,
            ":" .. command .. "<CR>",
            { noremap = true, silent = true }
        )
    end

    set_keymap("<leader>xb", "BuildIOS")
    set_keymap("<leader>xB", "BuildIOSDevice")
end

return M
