local ffiutil = require('ffi/util')
local logger = require('logger')

local Backend = require('Backend')

local JOB_POLLING_INTERVAL_SECONDS = 1

--- @class DownloadChapter
--- @field private source_id string
--- @field private manga_id string
--- @field private chapter_id string
--- @field private job_id string
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
  }
  setmetatable(o, self)
  self.__index = self

  local job_id = o:start()
  if job_id == nil then
    return nil
  end

  o.job_id = job_id

  return o
end

--- Starts the job. Should be called automatically when instantiating a job with `new()`.
---
--- @private
--- @return string|nil id The created job id, or false, if the job could not be created.
function DownloadChapter:start()
  local response = Backend.createDownloadChapterJob(self.source_id, self.manga_id, self.chapter_id)
  if response.type == 'ERROR' then
    logger.error('could not create download chapter job', response.message)

    return nil
  end

  return response.body
end

--- @return SuccessfulResponse<string>|ErrorResponse
function DownloadChapter:runUntilCompletion()
  while true do
    local response = Backend.getDownloadChapterJobDetails(self.job_id)
    if response.type == 'ERROR' then
      return response
    end

    local details = response.body

    if details.type == 'PENDING' then
      ffiutil.sleep(JOB_POLLING_INTERVAL_SECONDS)

      goto continue
    elseif details.type == 'COMPLETED' then
      return {
        type = 'SUCCESS',
        body = details.data
      }
    else
      return {
        type = 'ERROR',
        message = details.data.message
      }
    end

    ::continue::
  end
end

return DownloadChapter
