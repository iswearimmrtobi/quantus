-- name: Sudoku
-- kind: game
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Fill the grid so that every row, column and box holds 1 to 9.
-- about.de: Fülle das Gitter, sodass jede Zeile, Spalte und jeder Block 1 bis 9 enthält.
-- license: MIT

-- The grid is a list of 81 numbers, row by row, 0 for an empty cell: `puzzle` holds the
-- givens, `entries` what the player typed. A new game takes one of the puzzles below and
-- shuffles it (the digits, the rows within a band, the bands, the same for the columns, and a
-- transposition), which keeps it a Sudoku with one solution; generating one with a check for
-- a single solution would take longer than a callback may run.

local PUZZLES = {
  {
    "860009003491302500503040910000280090930010702048960030100800079300700050070000024",
    "056410030000700500847560902400006008000290003235000640600035470070021006019600200",
    "000029000250048370094030000900016207027005903600970500000764102402803006006000045",
    "000500009700920060005037042100259087000803090840070203370001008401380006208060004",
    "024006090003490800170000300010820600046759200000013574730001408008205060060008900",
    "780000602090480500125000084019803006073609048008120090057300200000910060000208003",
  },
  {
    "008000000350090060700001009081000206000130000439060015870000004002600800000004973",
    "000020504000000079053070280046500002900100005530007000604010900000009060020700053",
    "364501027009700060000040000090020700040070080701005300000000603030009008012800004",
    "070000600009162300100004590904500000026070030005000000041207000000301204050900010",
    "218000700000031000000000000032506900605000014080007060400060070063000002009713540",
    "905010000000900030864003900400000000020005080050230600518400309690000000200109500",
  },
  {
    "049560200100000000000000400010000002080005000000020731093870005000206003000090100",
    "007000040900000005600240031020000450300100000000002000001506098000800500006900002",
    "004200100000008000105000600901300000000094036000000010000005700800470095009060200",
    "000200000036000000008007045009080700300006000057300201900030510260090004000000000",
    "000070010067098002090002000000000201000067000073004090100009007008000060030000480",
    "004000035000001820000000000010020004520008000000506002903017408100000000600000370",
  },
}

local puzzle, entries, level, best, done, history
-- The time: seconds when the game was opened or started, and the tick's ms then.
local base, mark, last, elapsed

local function de() return lang() == "de" end

local function level_name(n)
  if de() then return ({"leicht", "mittel", "schwer"})[n] end
  return ({"easy", "medium", "hard"})[n]
end

local function labels()
  local undo = ""
  if #history > 0 and not done then undo = de() and "Zurück" or "Undo" end
  soft({de() and "Neu" or "New", level_name(level), undo})
end

-- The 81 numbers of a string of digits.
function parse(s)
  local p = {}
  for i = 1, 81 do p[i] = tonumber(s:sub(i, i)) end
  return p
end

-- Grid `p` rearranged: digit d becomes digits[d], row r of the result is row rows[r] of `p`,
-- column c column cols[c], and rows and columns swap when `transpose` is set.
function transform(p, digits, rows, cols, transpose)
  local out = {}
  for r = 1, 9 do
    for c = 1, 9 do
      local sr, sc = rows[r], cols[c]
      if transpose then sr, sc = cols[r], rows[c] end
      local v = p[(sr - 1) * 9 + sc]
      out[(r - 1) * 9 + c] = v == 0 and 0 or digits[v]
    end
  end
  return out
end

local function shuffle(list)
  for i = #list, 2, -1 do
    local j = math.random(i)
    list[i], list[j] = list[j], list[i]
  end
  return list
end

-- An order of the nine rows (or columns) that keeps each band of three together.
local function band_order()
  local order = {}
  for _, band in ipairs(shuffle({0, 1, 2})) do
    for _, k in ipairs(shuffle({1, 2, 3})) do order[#order + 1] = band * 3 + k end
  end
  return order
end

-- One of the puzzles of `n`, shuffled.
local function new_puzzle(n)
  local list = PUZZLES[n]
  local p = parse(list[math.random(#list)])
  return transform(p, shuffle({1, 2, 3, 4, 5, 6, 7, 8, 9}), band_order(), band_order(), math.random(2) == 1)
end

-- The cells whose digit repeats in their row, column or box: a table from index to true.
function conflicts(cells)
  local bad = {}
  local function check(group)
    local seen = {}
    for _, i in ipairs(group) do
      local v = cells[i]
      if v ~= 0 then
        if seen[v] then
          bad[i] = true
          bad[seen[v]] = true
        else
          seen[v] = i
        end
      end
    end
  end
  for k = 0, 8 do
    local row, col, box = {}, {}, {}
    local br, bc = k // 3 * 3, k % 3 * 3
    for j = 0, 8 do
      row[#row + 1] = k * 9 + j + 1
      col[#col + 1] = j * 9 + k + 1
      box[#box + 1] = (br + j // 3) * 9 + bc + j % 3 + 1
    end
    check(row)
    check(col)
    check(box)
  end
  return bad
end

-- Whether every cell holds a digit and none repeats.
function solved(cells)
  for i = 1, 81 do
    if cells[i] == 0 then return false end
  end
  return next(conflicts(cells)) == nil
end

-- The givens and the entries in one grid.
local function cells()
  local all = {}
  for i = 1, 81 do all[i] = puzzle[i] ~= 0 and puzzle[i] or entries[i] end
  return all
end

local function keep()
  save({level = level, puzzle = puzzle, entries = entries, time = elapsed, best = best})
end

local function start()
  puzzle = new_puzzle(level)
  entries = {}
  for i = 1, 81 do entries[i] = 0 end
  history = {}
  done = false
  base, mark, elapsed = 0, last, 0
  timer(1000)
end

-- Writes `v` (0 to clear) into cell `i`, unless it is a given.
local function set(i, v)
  if done or puzzle[i] ~= 0 or entries[i] == v then return end
  history[#history + 1] = {i = i, v = entries[i]}
  if #history > 200 then table.remove(history, 1) end
  entries[i] = v
  if solved(cells()) then
    done = true
    timer(0)
    if best[level] == 0 or elapsed < best[level] then best[level] = elapsed end
  end
  keep()
  labels()
end

local function valid(list, n)
  if type(list) ~= "table" or #list ~= n then return false end
  for _, v in ipairs(list) do
    if math.type(v) ~= "integer" then return false end
  end
  return true
end

function init()
  last = 0
  local saved = load()
  if type(saved) ~= "table" then saved = {} end
  level = (saved.level == 2 or saved.level == 3) and saved.level or 1
  best = valid(saved.best, 3) and saved.best or {0, 0, 0}
  input("grid", {cols = 9, rows = 9})
  if valid(saved.puzzle, 81) and valid(saved.entries, 81) then
    puzzle = saved.puzzle
    entries = saved.entries
    history = {}
    elapsed = math.type(saved.time) == "integer" and saved.time or 0
    base, mark = elapsed, 0
    done = solved(cells())
    if not done then timer(1000) end
  else
    start()
  end
  labels()
end

function tick(ms)
  last = ms
  if not done then elapsed = base + (ms - mark) // 1000 end
end

local function at_cursor()
  local c = cursor()
  return (c.row - 1) * 9 + c.col
end

-- OK or a tap counts the cell's digit up, for whoever plays by touch.
function pick(col, row)
  local i = (row - 1) * 9 + col
  set(i, (entries[i] + 1) % 10)
end

-- A new game on level `n`; asks first while a game is going on.
local function new_game(n)
  local running = false
  for i = 1, 81 do
    if entries[i] ~= 0 then running = true end
  end
  if done or not running or confirm(de() and "Neues Spiel?" or "New game?") then
    level = n
    start()
    keep()
    labels()
  end
end

function key(k)
  local d = tonumber(k)
  if d and d >= 1 and d <= 9 then
    set(at_cursor(), d)
  elseif k == "0" or k == "del" then
    set(at_cursor(), 0)
  elseif k == "soft1" then
    new_game(level)
  elseif k == "soft2" then
    new_game(level % 3 + 1)
  elseif k == "soft3" and #history > 0 and not done then
    local h = table.remove(history)
    entries[h.i] = h.v
    keep()
    labels()
  end
end

function leave() keep() end

local function clock(s)
  if s >= 3600 then return string.format("%d:%02d:%02d", s // 3600, s // 60 % 60, s % 60) end
  return string.format("%d:%02d", s // 60, s % 60)
end

local function side_panel()
  local s = side()
  local room = s.w - 8
  local x, y = s.x + 4, s.y + 4
  local small = text_small_height() + 4
  local lines = {}
  lines[#lines + 1] = {level_name(level), "muted"}
  lines[#lines + 1] = {clock(elapsed), "text", true}
  if best[level] > 0 then
    lines[#lines + 1] = {(de() and "Bestzeit " or "Best ") .. clock(best[level]), "muted"}
  end
  if done then
    lines[#lines + 1] = {de() and "Gelöst!" or "Solved!", "amber"}
  else
    lines[#lines + 1] = {de() and "1-9 setzen, 0 löschen" or "1-9 set, 0 clears", "muted"}
  end
  if s.h >= #lines * small + text_height() + 8 then
    -- Beside the board: one per line.
    for _, l in ipairs(lines) do
      if l[3] and text_width(l[1]) <= room then
        text(x, y, l[1], l[2])
        y = y + text_height() + 4
      elseif text_small_width(l[1]) <= room and y + small <= s.y + s.h then
        text_small(x, y, l[1], l[2])
        y = y + small
      end
    end
  else
    -- Below it: the level, the time and the state on one line.
    local line = level_name(level) .. "   " .. clock(elapsed)
    if done then line = line .. "   " .. lines[#lines][1] end
    if text_small_width(line) <= room and y + small <= s.y + s.h then text_small(x, y, line, done and "amber" or "text") end
  end
end

function draw()
  clear("background")
  local g = grid(9, 9)
  rect(g.x, g.y, g.w, g.h, "muted")
  local bad = conflicts(cells())
  local big = text_height() + 2
  for i = 1, 81 do
    local col, row = (i - 1) % 9 + 1, (i - 1) // 9 + 1
    local c = g:cell(col, row)
    -- Thin lines between cells, thick ones between the boxes and round the board.
    local l = col % 3 == 1 and 2 or 1
    local r = col == 9 and 2 or 0
    local t = row % 3 == 1 and 2 or 1
    local b = row == 9 and 2 or 0
    local x, y, w, h = c.x + l, c.y + t, c.w - l - r, c.h - t - b
    local given = puzzle[i] ~= 0
    rect(x, y, w, h, given and "surface" or "background")
    local v = given and puzzle[i] or entries[i]
    if v ~= 0 then
      local colour = given and "text" or "cyan"
      if bad[i] then colour = "error" elseif done then colour = given and "text" or "result" end
      local s = tostring(v)
      if big <= h then
        text(x + (w - text_width(s)) // 2, y + (h - text_height()) // 2, s, colour)
      else
        text_small(x + (w - text_small_width(s)) // 2, y + (h - text_small_height()) // 2, s, colour)
      end
    end
  end
  side_panel()
end
