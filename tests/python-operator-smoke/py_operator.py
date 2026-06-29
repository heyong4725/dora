from dora import DoraStatus


class Operator:
    def __init__(self):
        self.received = 0

    def on_event(self, dora_event, send_output):
        if dora_event["type"] == "INPUT":
            self.received += 1
            send_output("done", b"python-operator-ok", dora_event["metadata"])
            return DoraStatus.STOP
        return DoraStatus.CONTINUE
