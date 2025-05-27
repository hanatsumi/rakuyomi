local logger = require('logger')
local DownloadChapter = require('jobs/DownloadChapter')

--- @class DownloadScanlatorChapters: Job (Local job simulation, not a real backend job)
--- @field private source_id string
--- @field private manga_id string  
--- @field private amount number|nil
--- @field private scanlator string
--- @field private chapters table[]
--- @field private download_state table
local DownloadScanlatorChapters = {}

--- Creates a new `DownloadScanlatorChapters` job.
--- This is a LOCAL job that simulates the backend job interface
---
--- @param source_id string
--- @param manga_id string
--- @param chapters table[] All chapters from the manga
--- @param scanlator string The scanlator to filter by
--- @param amount number|nil
--- @return self|nil job A new job, or `nil`, if the job could not be created.
function DownloadScanlatorChapters:new(source_id, manga_id, chapters, scanlator, amount)
  local o = {
    source_id = source_id,
    manga_id = manga_id,
    chapters = chapters,
    scanlator = scanlator,
    amount = amount,
    download_state = {
      current_index = 1,
      completed = 0,
      total = 0,
      cancelled = false,
      current_job = nil,
      filtered_chapters = {}
    }
  }
  
  setmetatable(o, self)
  self.__index = self
  
  if not o:start() then
    return nil
  end
  
  return o
end

--- Starts the job. Should be called automatically when instantiating a job with `new()`.
---
--- @private
--- @return boolean success Whether the job started successfully.
function DownloadScanlatorChapters:start()
  -- Get chapters from selected scanlator
  local scanlator_chapters = {}
  for _, chapter in ipairs(self.chapters) do
    local chapter_scanlator = chapter.scanlator or "Unknown"
    if chapter_scanlator == self.scanlator then
      table.insert(scanlator_chapters, chapter)
    end
  end
  
  -- Find last read chapter (same logic as backend)
  local last_read_chapter = nil
  for _, chapter in ipairs(scanlator_chapters) do
    if chapter.read then
      last_read_chapter = chapter
      break  -- First read chapter found (= last read chronologically)
    end
  end
  
  -- Reverse list (like backend does with .rev())
  local reversed_chapters = {}
  for i = #scanlator_chapters, 1, -1 do
    table.insert(reversed_chapters, scanlator_chapters[i])
  end
  
  -- Skip chapters until after last read (like backend skip_while)
  local chapters_to_download = {}
  local skip_mode = (last_read_chapter ~= nil)
  
  for _, chapter in ipairs(reversed_chapters) do
    if skip_mode then
      -- Skip until we're past the last read chapter
      if last_read_chapter and chapter.chapter_num and last_read_chapter.chapter_num then
        if chapter.chapter_num <= last_read_chapter.chapter_num then
          goto continue  -- Still skipping
        else
          skip_mode = false  -- Found a chapter past the last read, stop skipping
        end
      end
    end
    
    -- Only add unread chapters
    if not chapter.read then
      table.insert(chapters_to_download, chapter)
    end
    
    ::continue::
  end
  
  if #chapters_to_download == 0 then
    return false  -- No chapters to download
  end
  
  -- Limit amount if specified
  if self.amount and self.amount < #chapters_to_download then
    local limited = {}
    for i = 1, self.amount do
      table.insert(limited, chapters_to_download[i])
    end
    chapters_to_download = limited
  end
  
  self.download_state.filtered_chapters = chapters_to_download
  self.download_state.total = #chapters_to_download
  
  return true
end

--- @alias PendingState { type: 'INITIALIZING' }|{ type: 'DOWNLOADING', downloaded: number, total: number }
--- @return SuccessfulResponse<nil>|PendingResponse<PendingState>|ErrorResponse
function DownloadScanlatorChapters:poll()
  if self.download_state.cancelled then
    return { type = 'SUCCESS' }  -- Treat cancellation as success
  end
  
  if self.download_state.current_index > self.download_state.total then
    return { type = 'SUCCESS' }
  end
  
  -- Start with INITIALIZING state for first poll
  if self.download_state.current_index == 1 and not self.download_state.current_job then
    -- Return INITIALIZING for first call, then create job on next poll
    return { 
      type = 'PENDING', 
      body = { 
        type = 'INITIALIZING'
      } 
    }
  end
  
  -- Download next chapter if needed
  if not self.download_state.current_job then
    local chapter = self.download_state.filtered_chapters[self.download_state.current_index]
    self.download_state.current_job = DownloadChapter:new(
      chapter.source_id, 
      chapter.manga_id, 
      chapter.id, 
      chapter.chapter_num
    )
    
    if not self.download_state.current_job then
      return { type = 'ERROR', message = 'Could not create download job for chapter' }
    end
  end
  
  -- Check job status
  local result = self.download_state.current_job:poll()
  
  if result.type == 'SUCCESS' then
    -- Mark chapter as downloaded
    self.download_state.filtered_chapters[self.download_state.current_index].downloaded = true
    
    self.download_state.completed = self.download_state.completed + 1
    self.download_state.current_index = self.download_state.current_index + 1
    self.download_state.current_job = nil
    
    if self.download_state.current_index > self.download_state.total then
      return { type = 'SUCCESS' }
    end
  elseif result.type == 'ERROR' then
    return result
  end
  
  return { 
    type = 'PENDING', 
    body = { 
      type = 'DOWNLOADING', 
      downloaded = self.download_state.completed, 
      total = self.download_state.total 
    } 
  }
end

--- @return SuccessfulResponse<nil>|ErrorResponse
function DownloadScanlatorChapters:runUntilCompletion()
  while true do
    local result = self:poll()
    if result.type ~= 'PENDING' then
      return result
    end
    require("ffi/util").sleep(0.1)
  end
end

--- Request cancellation of the job
--- @return boolean success Whether the cancellation request was successful
function DownloadScanlatorChapters:requestCancellation()
  self.download_state.cancelled = true
  return true
end

return DownloadScanlatorChapters