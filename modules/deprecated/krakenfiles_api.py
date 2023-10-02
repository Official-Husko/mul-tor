import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Krakenfiles"

# This version is currently not in use because the API docs are outdated. Following the API rules results in a 500 error.

class Krakenfiles:
    
    def Uploader(file, proxy_list, user_agents):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            server_url = sites_data_dict[site]["server_url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
            
            characters = string.ascii_lowercase + string.digits + string.ascii_uppercase
            file_id = ''.join(random.choice(characters) for i in range(10))
            access_code = ''.join(random.choice(characters) for i in range(7))

            if proxy_list == []:
                server_req = requests.get(url=server_url, headers={"User-Agent": ua})
            else:
                server_req = requests.get(url=server_url, headers={"User-Agent": ua}, proxies=random.choice(proxy_list))

            server_response = server_req.json()
            print(server_response)
            upload_url = server_response.get("data", {}).get("url", "")
            print(upload_url)
            token = server_response.get("data", {}).get("serverAccessToken", "")
            print(token)

            if calc_size == "OK":
                data = {
                    "serverAccessToken": token
                }
                form_data = {
                    'file': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                if proxy_list == []:
                    raw_req = requests.post(url=upload_url, data=data, files=form_data, headers={"User-Agent": ua})
                else:
                    raw_req = requests.post(url=upload_url, files=form_data, headers={"User-Agent": ua}, proxies=random.choice(proxy_list))

                if raw_req.status_code == 200:
                    return {"status": "ok", "file_name": file_name, "file_url": download_url_base + file_id, "site": site}
                else:
                    raise Exception(f"Status code: {raw_req.status_code}")
            else:
                return {"status": "size_error", "file_name": file_name, "site": site, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "site": site, "exception": str(e), "extra": raw_req.content}