function NeogitConfig()
    local neogit = require("neogit")
    neogit.setup({
        disable_commit_confirmation = true,
        disable_context_highlighting = true,
    })
end
