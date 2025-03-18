import pytest
from pytest import StashKey, CollectReport, Item
from _pytest.nodes import Item
from _pytest.runner import CallInfo
from typing import Dict, Generator

phase_report_key = StashKey[Dict[str, CollectReport]]()

@pytest.hookimpl(wrapper=True, tryfirst=True)
def pytest_runtest_makereport(item: Item, call: CallInfo) -> Generator[None, CollectReport, CollectReport]:
    # execute all other hooks to obtain the report object
    rep = yield

    if rep.when is None:
        raise ValueError("Report object must have a 'when' attribute")

    # store test results for each phase of a call, which can
    # be "setup", "call", "teardown"
    item.stash.setdefault(phase_report_key, {})[rep.when] = rep

    return rep
