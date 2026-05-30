local M = {}

local plugin_root = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")
-- core/ is a sibling of nvim/, so go up one more level
local project_root = vim.fn.fnamemodify(plugin_root, ":h")

local defaults = {
	search_binary = project_root .. "/core/target/release/core",
	vault_path = "~/Projects/vault/",
}

M.config = {}

function M.setup(opts)
	M.config = vim.tbl_deep_extend("force", defaults, opts or {})
end

local function get_tags()
	local binary = vim.fn.expand(M.config.search_binary)
	local vault = vim.fn.expand(M.config.vault_path)
	local raw = vim.fn.system({ binary, "tags", vault })
	if vim.v.shell_error ~= 0 then
		vim.notify("obsidian-search tags failed:\n" .. raw, vim.log.levels.ERROR)
		return {}
	end
	return vim.json.decode(raw)
end

function M.clear_tag_cache()
	local binary = vim.fn.expand(M.config.search_binary)
	local vault = vim.fn.expand(M.config.vault_path)
	vim.fn.system({ binary, "clear-tag-cache", vault })
	vim.notify("Tag cache cleared.", vim.log.levels.INFO)
end

function M.search_tasks(opts)
	local pickers = require("telescope.pickers")
	local finders = require("telescope.finders")
	local conf = require("telescope.config").values

	opts = opts or {}
	local command = { vim.fn.expand(M.config.search_binary), "todos", vim.fn.expand(M.config.vault_path) }

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

	local raw = vim.fn.system(command)
	if vim.v.shell_error ~= 0 then
		vim.notify("obsidian-search todos failed:\n" .. raw, vim.log.levels.ERROR)
		return
	end
	local response = vim.json.decode(raw)

	pickers
		.new(opts, {
			prompt_title = "Search Notes",
			finder = finders.new_table({
				results = response,
				entry_maker = function(entry)
					return {
						value = entry,
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

function M.requires(opts)
	local pickers = require("telescope.pickers")
	local finders = require("telescope.finders")
	local previewers = require("telescope.previewers")
	local conf = require("telescope.config").values
	local actions = require("telescope.actions")
	local action_state = require("telescope.actions.state")

	opts = opts or {}

	local binary = vim.fn.expand(M.config.search_binary)
	local vault = vim.fn.expand(M.config.vault_path)
	local raw = vim.fn.system({ binary, "requires", vault })
	if vim.v.shell_error ~= 0 then
		vim.notify("obsidian-search requires failed:\n" .. raw, vim.log.levels.ERROR)
		return
	end
	local by_person = vim.json.decode(raw)

	-- Build a flat list of { person, tasks } sorted by person name
	local people = {}
	for person, tasks in pairs(by_person) do
		table.insert(people, { person = person, tasks = tasks })
	end
	table.sort(people, function(a, b)
		return a.person < b.person
	end)

	-- Preview: show all tasks for a person as plain text
	local person_previewer = previewers.new_buffer_previewer({
		title = "Tasks",
		define_preview = function(self, entry)
			local lines = {}
			for _, task in ipairs(entry.value.tasks) do
				table.insert(lines, task.display)
				table.insert(lines, "  " .. task.path .. ":" .. task.lnum)
				table.insert(lines, "")
			end
			vim.api.nvim_buf_set_lines(self.state.bufnr, 0, -1, false, lines)
		end,
	})

	pickers
		.new(opts, {
			prompt_title = "Requires — People",
			finder = finders.new_table({
				results = people,
				entry_maker = function(entry)
					return {
						value = entry,
						display = entry.person .. " (" .. #entry.tasks .. ")",
						ordinal = entry.person,
					}
				end,
			}),
			previewer = person_previewer,
			sorter = conf.generic_sorter(opts),
			attach_mappings = function(bufnr, _)
				actions.select_default:replace(function()
					actions.close(bufnr)
					local selection = action_state.get_selected_entry()
					local tasks = selection.value.tasks
					local person = selection.value.person

					pickers
						.new(opts, {
							prompt_title = "Requires — " .. person,
							finder = finders.new_table({
								results = tasks,
								entry_maker = function(task)
									return {
										value = task,
										display = task.display,
										ordinal = task.ordinal,
										path = task.path,
										lnum = task.lnum,
									}
								end,
							}),
							previewer = conf.grep_previewer(opts),
							sorter = conf.generic_sorter(opts),
						})
						:find()
				end)
				return true
			end,
		})
		:find()
end

function M.heading_check(opts)
	local pickers = require("telescope.pickers")
	local finders = require("telescope.finders")
	local conf = require("telescope.config").values

	opts = opts or {}
	local binary = vim.fn.expand(M.config.search_binary)
	local vault = vim.fn.expand(M.config.vault_path)
	local raw = vim.fn.system({ binary, "heading-check", vault })
	if vim.v.shell_error ~= 0 then
		vim.notify("obsidian-search heading-check failed:\n" .. raw, vim.log.levels.ERROR)
		return
	end
	local results = vim.json.decode(raw)

	pickers
		.new(opts, {
			prompt_title = "Heading Mismatches",
			finder = finders.new_table({
				results = results,
				entry_maker = function(entry)
					local heading = (entry.heading ~= vim.NIL and entry.heading) or "(no h1 heading)"
					return {
						value = entry,
						display = entry.filename .. "  →  " .. heading,
						ordinal = entry.filename .. " " .. heading,
						path = entry.path,
						lnum = 1,
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
	local tags = get_tags()
	pickers
		.new(opts, {
			finder = finders.new_table({ results = tags }),
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
