local ffiutil = require('ffi/util')
local logger = require('logger')

local Backend = require('Backend')

local JOB_POLLING_INTERVAL_SECONDS = 1

--- @class DownloadChapter
--- @field private source_id string
--- @field private manga_id string
--- @field private chapter_id string
--- @field private job_id string
--- @field private result SuccessfulResponse<string>|ErrorResponse|nil
local DownloadChapter = {}

--- Creates a new `DownloadChapter` job.
---
--- @param source_id string
--- @param manga_id string
--- @param chapter_id string
--- @return self|nil job A new `DownloadChapter` job, or `nil`, if the job could not be created.
function DownloadChapter:new(source_id, manga_id, chapter_id)
  local o = {
    source_id = source_id,
    manga_id = manga_id,
    chapter_id = chapter_id,
    result = nil,
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
function DownloadChapter:start()
  local response = Backend.createDownloadChapterJob(self.source_id, self.manga_id, self.chapter_id)
  if response.type == 'ERROR' then
    logger.error('could not create download chapter job', response.message)

    return false
  end

  self.job_id = response.body

  return true
end

--- Peeks the current job competion. Returns `nil` if the job hasn't completed.
---
--- @return SuccessfulResponse<string>|ErrorResponse|nil
function DownloadChapter:poll()
  if self.result ~= nil then
    return self.result
  end

  local response = Backend.getDownloadChapterJobDetails(self.job_id)
  if response.type == 'ERROR' then
    self.result = response

    return self.result
  end

  local details = response.body

  if details.type == 'PENDING' then
    return nil
  elseif details.type == 'COMPLETED' then
    self.result = {
      type = 'SUCCESS',
      --- @diagnostic disable-next-line: assign-type-mismatch
      body = details.data
    }
  else
    self.result = {
      type = 'ERROR',
      message = details.data.message
    }
  end

  return self.result
end

--- Runs this job up to completion, returning the result.
---
--- @return SuccessfulResponse<string>|ErrorResponse
function DownloadChapter:runUntilCompletion()
  while true do
    local result = self:poll()

    if result ~= nil then
      return result
    end

    ffiutil.sleep(JOB_POLLING_INTERVAL_SECONDS)
  end
end

return DownloadChapter
