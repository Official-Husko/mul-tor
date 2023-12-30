import requests
import os
import random
import string
import re

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "UsersDrive"

class UsersDrive:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "which one of you maggots ate the fucking request huh?"
        status = "terrible"
        try:
            ua = random.choice(user_agents)
            download_url_base = sites_data_dict[site]["download_url_base"]
            normal_url = sites_data_dict[site]["api_url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":

                raw_req = requests.get(url=normal_url, headers={"User-Agent": ua}, proxies=proxies, timeout=50)

                pattern = r'https://d\d+\.userdrive\.me/cgi-bin/upload\.cgi\?upload_type=file'
                match = re.search(pattern, raw_req.text)

                if match:
                    upload_url = match.group(0)
                else:
                    raise Exception("Server URL Missing. Report this!")

                data = {
                    "utype": "anon",
                    "keepalive": 1,
                }
                form_data = {
                    'file_0': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, data=data, files=form_data, headers=headers, proxies=proxies, stream=True)

                response = raw_req.json()
                status = response[0].get("file_status", "terrible")
                file_id = response[0].get("file_code", "No_Code :(")

                if raw_req.status_code == 200 and status == "OK":
                    return {"status": "ok", "file_name": file_name, "file_url": download_url_base.format(file_id=file_id)}
                else:
                    raise Exception(f"Status code: {raw_req.status_code}")
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": f"{status} {raw_req}"}

