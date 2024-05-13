# Import Standard Libraries
import json

# Import Third-Party Libraries

# Import Local Libraries

class PresetManager:

    def __init__(self, available: list[str], preset_name: str):
        self.available_hosters = available
        self.preset_name = preset_name

    def _load(self):
        try:
            with open(f"presets/{self.preset_name}", "r", encoding="utf-8") as preset_file:
                preset = json.load(preset_file)
                preset_sites = preset.get("sites", [])
                link_format = preset.get("link-format", "")

                final_sites = []

                for whitelisted_site in preset_sites:
                    if whitelisted_site in self.available_hosters:
                        final_sites.append(whitelisted_site)

            return final_sites, link_format

        except Exception as e:
            print(e)
    
    def _update_preset(self):
        pass # TODO: implement update preset code
