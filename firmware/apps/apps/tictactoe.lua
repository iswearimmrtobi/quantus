-- name: Tic tac toe
-- name.de: Tic Tac Toe
-- kind: game
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Three in a row, against the calculator or a second player.
-- about.de: Drei in einer Reihe, gegen den Rechner oder zu zweit.
-- license: MIT

-- The board is a list of nine cells, row by row: "", "X" or "O". X is the player, O the
-- computer or the second player.

local board, starter, turn, computer, hard, score, over, won_line

-- The three cells of each line.
local LINES = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}, {1, 4, 7}, {2, 5, 8}, {3, 6, 9}, {1, 5, 9}, {3, 5, 7}}

local function de() return lang() == "de" end

local function other(mark) return mark == "X" and "O" or "X" end

local function keep()
  save({computer = computer, hard = hard, score = score})
end

-- The soft keys: Neu, the mode, the level (only against the computer), each showing what is set.
local function labels()
  local mode, level = "", ""
  if computer then
    mode = de() and "Rechner" or "CPU"
    if hard then level = de() and "schwer" or "hard" else level = de() and "leicht" or "easy" end
  else
    mode = de() and "2 Spieler" or "2 players"
  end
  soft({de() and "Neu" or "New", mode, level})
end

-- The line `b` has three of a kind in, or nil.
local function won(b)
  for _, line in ipairs(LINES) do
    local a = b[line[1]]
    if a ~= "" and a == b[line[2]] and a == b[line[3]] then return line end
  end
  return nil
end

local function full(b)
  for i = 1, 9 do
    if b[i] == "" then return false end
  end
  return true
end

-- The free cells, the centre first, then the corners.
local function ordered(b)
  local cells = {}
  for _, i in ipairs({5, 1, 3, 7, 9, 2, 4, 6, 8}) do
    if b[i] == "" then cells[#cells + 1] = i end
  end
  return cells
end

-- The free cell of a line with two of `m` in it, or nil.
local function open_end(b, line, m)
  local x, y, z = b[line[1]], b[line[2]], b[line[3]]
  if x == m and y == m and z == "" then return line[3] end
  if x == m and z == m and y == "" then return line[2] end
  if y == m and z == m and x == "" then return line[1] end
  return nil
end

-- A free cell that gives `m` a line, or nil.
local function completing(b, m)
  for _, line in ipairs(LINES) do
    local cell = open_end(b, line, m)
    if cell then return cell end
  end
  return nil
end

-- How many lines `m` could complete with one more mark.
local function threats(b, m)
  local n = 0
  for _, line in ipairs(LINES) do
    if open_end(b, line, m) then n = n + 1 end
  end
  return n
end

-- The free cells where a mark of `m` makes two threats at once.
local function forks(b, m)
  local cells = {}
  for _, i in ipairs(ordered(b)) do
    b[i] = m
    if threats(b, m) >= 2 then cells[#cells + 1] = i end
    b[i] = ""
  end
  return cells
end

-- The best cell for `me` on board `b`, by the rules that never lose: win, block, fork, block a
-- fork, the centre, the corner opposite the other's, a corner, a side. They take a few thousand
-- instructions, where a search of the game tree would take seconds on the handheld.
function best(b, me)
  local you = other(me)
  local cell = completing(b, me) or completing(b, you)
  if cell then return cell end
  local mine = forks(b, me)
  if #mine > 0 then return mine[1] end
  local theirs = forks(b, you)
  if #theirs == 1 then return theirs[1] end
  if #theirs > 1 then
    -- A threat of my own, so that the other must block, where blocking makes no fork.
    for _, i in ipairs(ordered(b)) do
      b[i] = me
      local block = completing(b, me)
      local good = false
      if block then
        b[block] = you
        good = threats(b, you) < 2
        b[block] = ""
      end
      b[i] = ""
      if good then return i end
    end
  end
  if b[5] == "" then return 5 end
  for _, pair in ipairs({{1, 9}, {9, 1}, {3, 7}, {7, 3}}) do
    if b[pair[1]] == you and b[pair[2]] == "" then return pair[2] end
  end
  return ordered(b)[1]
end

-- Sets the mark of whose turn it is on cell `i`, and sees whether the round is over.
local function play(i)
  if over or board[i] ~= "" then return end
  board[i] = turn
  local line = won(board)
  if line then
    over = true
    won_line = line
    if turn == "X" then score.x = score.x + 1 else score.o = score.o + 1 end
    keep()
    return
  end
  if full(board) then
    over = true
    score.draw = score.draw + 1
    keep()
    return
  end
  turn = other(turn)
end

-- The computer's move, when it is its turn: a free cell at random, or on *schwer* the best.
local function cpu()
  if not computer or over or turn ~= "O" then return end
  if hard then
    play(best(board, "O"))
  else
    local cells = ordered(board)
    play(cells[math.random(#cells)])
  end
end

-- A new round, `first` beginning.
local function new_round(first)
  board = {"", "", "", "", "", "", "", "", ""}
  starter = first
  turn = first
  over = false
  won_line = nil
  cpu()
end

function init()
  board = {"", "", "", "", "", "", "", "", ""}
  starter = "X"
  turn = "X"
  -- The mode, the level and the score as they were left.
  local saved = load()
  if type(saved) ~= "table" then saved = {} end
  computer = saved.computer ~= false
  hard = saved.hard == true
  score = type(saved.score) == "table" and saved.score or {x = 0, o = 0, draw = 0}
  over = false
  won_line = nil
  input("grid", {cols = 3, rows = 3, digits = true})
  labels()
end

function leave() keep() end

function pick(col, row)
  -- Any key after a round starts the next one, the other side beginning.
  if over then
    new_round(other(starter))
    return
  end
  if computer and turn == "O" then return end
  play((row - 1) * 3 + col)
  cpu()
end

function key(k)
  if k == "soft1" then
    score = {x = 0, o = 0, draw = 0}
    new_round("X")
    keep()
  elseif k == "soft2" then
    computer = not computer
    labels()
    keep()
    cpu()
  elseif k == "soft3" then
    hard = not hard
    labels()
    keep()
  elseif over then
    new_round(other(starter))
  end
end

-- Whose turn it is, or how the round ended, and the score.
local function side_panel()
  local s = side()
  local status
  if over then
    if not won_line then
      status = de() and "Unentschieden" or "A draw"
    else
      local winner = board[won_line[1]]
      if computer and winner == "O" then
        status = de() and "Der Rechner gewinnt" or "The CPU wins"
      else
        status = winner .. (de() and " gewinnt" or " wins")
      end
    end
  else
    status = (de() and "Am Zug: " or "To move: ") .. turn
  end
  local step = text_small_height() + 6
  local room = s.w - 8
  local y = s.y + 4
  local colour = over and "amber" or "text"
  -- The status in the large font where it fits, else the small one.
  if text_width(status) <= room then
    text(s.x + 4, y, status, colour)
    y = y + text_height() + 6
  else
    text_small(s.x + 4, y, status, colour)
    y = y + step
  end
  local draws = de() and "Remis" or "Draws"
  local line = "X " .. score.x .. "   O " .. score.o .. "   " .. draws .. " " .. score.draw
  if text_small_width(line) <= room then
    text_small(s.x + 4, y, line, "muted")
  else
    text_small(s.x + 4, y, score.x .. " : " .. score.o, "muted")
  end
  y = y + step
  local hint
  if over then
    hint = de() and "Taste: nächste Runde" or "Any key: next round"
  else
    hint = de() and "Pfeile und OK, oder 1-9" or "Arrows and OK, or 1-9"
  end
  if y + step <= s.y + s.h and text_small_width(hint) <= room then
    text_small(s.x + 4, y, hint, "muted")
  end
end

-- The column and row of cell `i`, from 1.
local function place(i) return (i - 1) % 3 + 1, (i - 1) // 3 + 1 end

function draw()
  clear("background")
  local g = grid(3, 3)
  local gap = g.size // 16 + 2
  for i = 1, 9 do
    local c = g:cell(place(i))
    local mark = board[i]
    round(c.x + gap, c.y + gap, c.w - 2 * gap, c.h - 2 * gap, 8, "surface")
    local m = c.w // 4
    if mark == "X" then
      line(c.x + m, c.y + m, c.x + c.w - m, c.y + c.h - m, "cyan", 6)
      line(c.x + c.w - m, c.y + m, c.x + m, c.y + c.h - m, "cyan", 6)
    elseif mark == "O" then
      circle(c.x + m, c.y + m, c.w - 2 * m, "amber")
      circle(c.x + m + 6, c.y + m + 6, c.w - 2 * m - 12, "surface")
    end
  end
  if won_line then
    local a = g:cell(place(won_line[1]))
    local b = g:cell(place(won_line[3]))
    line(a.x + a.w // 2, a.y + a.h // 2, b.x + b.w // 2, b.y + b.h // 2, "text", 6)
  end
  side_panel()
end
