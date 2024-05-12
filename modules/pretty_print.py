from termcolor import colored

class PrettyPrint:
    def __init__(self):
        self.major = str(colored(f"[{colored('!!!', 'red')}]"))
        self.error = str(colored(f"[{colored('!', 'red')}]"))
        self.warning = str(colored(f"[{colored('!', 'yellow')}]"))
        self.info = str(colored(f"[{colored('i', 'light_blue')}]"))
        self.ok = str(colored(f"[{colored('+', 'green')}]"))
        self.debug = str(colored(f"[{colored('dbg', 'blue')}]"))
