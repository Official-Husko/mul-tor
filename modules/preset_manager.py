# Import Standard Libraries
import json

# Import Third-Party Libraries

# Import Local Libraries
from .logger import Logger

class PresetManager:

    def __init__(self, preset_name: str) -> None:
        self.preset_name = preset_name

    def _load(self) -> tuple[list[str], str]:
        try:
            with open(f"presets/{self.preset_name}", "r", encoding="utf-8") as preset_file:
                preset: dict = json.load(preset_file)
                preset_sites: list[str] = preset.get("sites", [])
                link_format: str = preset.get("link-format", None)

            return preset_sites, link_format

        except Exception as e:
            logger_instance = Logger(message=f"Failed to load preset {self.preset_name}", extra=str(e), severity="error")
            logger_instance.log_event()
    
    def _update_preset(self) -> None:
        pass # TODO: implement update preset code
