import os
import json

class Preset_Manager:

    def loader(available, preset_name):
        if not os.path.exists("presets"):
            os.mkdir("presets")
        if not os.path.exists(f"presets/readme.txt"):
            with open("presets\\readme.txt", "a") as readme:
                text = "To create your own preset visit the wiki here: https://github.com/Official-Husko/mul-tor/wiki/Preset-Configuration"
                readme.write(text)

        print("loader started")
        print(os.getcwd())
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