import os
import json

class Preset_Manager:

    def loader(available, preset_name):
        try:
            with open(f"presets/{preset_name}", "r") as preset_file:
                preset = json.load(preset_file)
                preset_sites = preset.get("sites", [])
                link_format = preset.get("link-format", "")

                final_sites = []

                for whitelisted_site in preset_sites:
                    if whitelisted_site in available:
                        final_sites.append(whitelisted_site)

            return final_sites, link_format

        except Exception as e:
            print(e)