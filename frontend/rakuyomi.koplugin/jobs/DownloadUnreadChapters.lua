local logger = require('logger')

local Backend = require('Backend')
local Job = require('jobs/Job')

--- @class DownloadUnreadChapters: Job
--- @field private source_id string
--- @field private manga_id string
--- @field private amount number|nil
--- @field private job_id string
local DownloadUnreadChapters = Job:extend()

--- Creates a new `DownloadChapter` job.
---
--- @param source_id string
--- @param manga_id string
--- @param amount number|nil
--- @return self|nil job A new `DownloadChapter` job, or `nil`, if the job could not be created.
function DownloadUnreadChapters:new(source_id, manga_id, amount)
  local o = {
    source_id = source_id,
    manga_id = manga_id,
    amount = amount,
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
--- @privata
--- @return boolean success Whether the job started successfully.
function DownloadUnreadChapters:start()
  local response = Backend.createDownloadUnreadChaptersJob(self.source_id, self.manga_id, self.amount)
  if response.type == 'ERROR' then
    logger.error('could not create download unread chapters job', response.message)

    return false
  end

  self.job_id = response.body

  return true
end

--- @alias PendingState { type: 'INITIALIZING' }|{ type: 'DOwNLOADING', downloaded: number, total: number }

--- @return SuccessfulResponse<nil>|PendingResponse<PendingState>|ErrorResponse
function DownloadUnreadChapters:poll()
  return Job.poll(self)
end

--- @return SuccessfulResponse<nil>|ErrorResponse
function DownloadUnreadChapters:runUntilCompletion()
  return Job.runUntilCompletion(self)
end

return DownloadUnreadChapters
