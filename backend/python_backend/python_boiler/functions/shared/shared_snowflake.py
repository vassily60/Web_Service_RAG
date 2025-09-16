import snowflake.connector
from ast import literal_eval
from snowflake.connector import DictCursor
from botocore.exceptions import ClientError
from functions.shared.shared_functions import *

#credit for database
def generateSnowConnection(snowFlakeDict):
    ctx = snowflake.connector.connect(
        user=str(snowFlakeDict["user"]),
        password=str(snowFlakeDict["password"]),
        account=str(snowFlakeDict["account"]),
        warehouse=str(snowFlakeDict["warehouse"])
    )
    return ctx



