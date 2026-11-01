"""Internal entry point invoked by the desktop scheduler."""

import argparse

from .reminders import _deliver

parser = argparse.ArgumentParser()
parser.add_argument("reminder_id")
_deliver(parser.parse_args().reminder_id)
