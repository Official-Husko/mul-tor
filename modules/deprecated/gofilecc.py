import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "GoFileCC"

"""

"GoFileCC": {
    "apiKey": False,
    "url": "https://gofile.cc/api/v1/upload",
    "api_url": "https://gofile.cc/",
    "download_url_base": "https://gofile.cc/",
    "size_limit": 7,
    "size_unit": "GB"
},

"""

class GoFileCC:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                form_data = {
                    'file': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies, stream=True)

                response = raw_req.json()
                download_url = response.get("data", {}).get("file", {}).get("url", {}).get("short", "")

                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

