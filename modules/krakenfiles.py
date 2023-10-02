import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Krakenfiles"

class Krakenfiles:
    
    def Uploader(file, proxy_list, user_agents):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["site_upload_url"].format(number=random.randint(1, 10))
            download_url_base = sites_data_dict[site]["download_url_base"]
            server_url = sites_data_dict[site]["server_url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                form_data = {
                    'files[]': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies)

                response = raw_req.json()

                if raw_req.status_code == 200:
                    url = response.get("files", [{}])[0].get("url")
                    return {"status": "ok", "file_name": file_name, "file_url": download_url_base + url, "site": site}
                else:
                    raise Exception(f"Status code: {raw_req.status_code}")
            else:
                return {"status": "size_error", "file_name": file_name, "site": site, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "site": site, "exception": str(e), "extra": raw_req.content}