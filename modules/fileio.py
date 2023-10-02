import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Fileio"

class Fileio:
    def Uploader(file, proxy_list, user_agents):
        """
        Uploads a file to a specified site using random user agents and proxies.

        Args:
            file (str): The path to the file to be uploaded.
            proxy_list (list): A list of proxy URLs.
            user_agents (list): A list of user agent strings.

        Returns:
            dict: A dictionary containing the status, file name, file URL, and site.

        Raises:
            Exception: If an error occurs during the upload process.
        """
        try:
            # Select a random user agent
            ua = random.choice(user_agents)

            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Get the upload URL and size limit from the site data dictionary
            upload_url = sites_data_dict[site]["url"].format(file_name=file_name)

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Set the user agent header
            headers = {"User-Agent": ua}

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                # Prepare the form data for file upload
                form_data = {
                    'file': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }

                # Send the upload request with the form data, headers, and proxies
                raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies)

                # Parse the response JSON and get the download URL
                token = raw_req.headers.get("x-file-io-anonymous-access-token", "")
                req = raw_req.json()
                download_url = req.get("link", "")
                file_id = req.get("key", "")

                # Prepare the json data to add extra data to the upload
                json_data = {
                    "title": file_name,
                    "description":"Uploaded using Mul-Tor on GitHub!",
                    "expires":"2M"
                }

                # Set the user agent header with additional Authorization header
                headers = {"User-Agent": ua, "Authorization": f"Bearer {token}"}

                # Get the patch URL from the site data dictionary
                patch_url = sites_data_dict[site]["patch_url"].format(file_id=file_id)

                # Send the patch request
                patch_req = requests.patch(url=patch_url, json=json_data, headers=headers, proxies=proxies)

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url, "site": site}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "site": site, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "site": site, "exception": str(e), "extra": raw_req.content}
