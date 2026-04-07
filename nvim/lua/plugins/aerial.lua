return {
    {
        "stevearc/aerial.nvim",
        -- remove this override after AstroNvim stops pinning Aerial to the older ^2.2 line
        -- and Lazy resolves to a nvim 0.12-compatible release on its own
        commit = "645d108a5242ec7b378cbe643eb6d04d4223f034",
        version = false,
    },
}
