local M = {}

local plugin_root = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")
-- core/ is a sibling of nvim/, so go up one more level
local project_root = vim.fn.fnamemodify(plugin_root, ":h")

local defaults = {
	search_binary = project_root .. "/core/target/release/core",
	vault_path = "~/Projects/vault/",
	tags = {
		"#NextAction",
		"#RequiresMother",
		"#RequiresFather",
		"#Errands",
		"#OnTheGo",
		"#Active",
		"#Waiting",
		"#Mindless",
		"#Refactoring",
		"#Createing",
		"#Producing",
		"#Enjoyment",
	},
}

M.config = {}

function M.setup(opts)
	M.config = vim.tbl_deep_extend("force", defaults, opts or {})
end

function M.search_tasks(opts)
	local pickers = require("telescope.pickers")
	local finders = require("telescope.finders")
	local conf = require("telescope.config").values

	opts = opts or {}
	local command = { vim.fs.abspath(M.config.search_binary) }
	table.insert(command, vim.fs.abspath(M.config.vault_path))

	if opts.include then
		for _, tag in ipairs(opts.include) do
			table.insert(command, "-i")
			table.insert(command, tag)
		end
	end

	if opts.any then
		for _, tag in ipairs(opts.any) do
			table.insert(command, "-a")
			table.insert(command, tag)
		end
	end

	if opts.exclude then
		for _, tag in ipairs(opts.exclude) do
			table.insert(command, "-e")
			table.insert(command, tag)
		end
	end

	local command_and_args = table.concat(command, " ")
	-- vim.print("obsidian-search command: " .. command_and_args)
	local raw = vim.fn.system(command_and_args)
	-- vim.print("obsidian-search response: " .. raw)
	local response = vim.json.decode(raw)

	pickers
		.new(opts, {
			prompt_title = "Search Notes",
			finder = finders.new_table({
				results = response,
				entry_maker = function(entry)
					return {
						value = entry,
						-- Newlines in display crash the cursor position
						display = entry.display,
						ordinal = entry.ordinal,
						path = entry.path,
						lnum = entry.lnum,
					}
				end,
			}),
			previewer = conf.grep_previewer(opts),
			sorter = conf.generic_sorter(opts),
		})
		:find()
end

function M.add_tags(opts)
	local pickers = require("telescope.pickers")
	local finders = require("telescope.finders")
	local conf = require("telescope.config").values
	local actions = require("telescope.actions")
	local action_state = require("telescope.actions.state")

	opts = opts or {}
	pickers
		.new(opts, {
			finder = finders.new_table({ results = M.config.tags }),
			sorter = conf.generic_sorter(opts),
			attach_mappings = function(bufnr, _)
				actions.select_default:replace(function()
					actions.close(bufnr)
					local selection = action_state.get_selected_entry()
					local current_line = vim.api.nvim_get_current_line()
					vim.api.nvim_set_current_line(current_line .. " " .. selection[1])
				end)
				return true
			end,
		})
		:find()
end

return M
