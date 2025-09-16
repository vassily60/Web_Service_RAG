INSERT INTO document_library.users (user_uuid,user_name,user_email,department,sso_unique_id,creation_date,created_by,updated_date,updated_by,"comments") VALUES
	 ('9e0ad907-cdf7-484d-96d1-08071a79da19','bfoucque','bfoucque@palo-it.com','PALO','bfoucque@palo-it.com',NULL,NULL,NULL,NULL,NULL);
INSERT INTO document_library.security_groups (security_group_uuid,security_group_name,security_group_description,security_group_type,creation_date,created_by,updated_date,updated_by,"comments") VALUES
	 ('4b116a72-c3af-4561-8523-3cd9ebe572fd','ALL','All access','group',NULL,NULL,NULL,NULL,NULL),
	 ('d3e855ea-8541-408c-b732-6d5b60db3a77','CONTRACT','only contract access','group',NULL,NULL,NULL,NULL,NULL);
INSERT INTO document_library.user_security_groups (user_security_group_uuid,security_group_uuid,user_uuid,creation_date,created_by,updated_date,updated_by,"comments") VALUES
	 ('54a31ced-1b95-437f-823b-c0b6f9f416d7','4b116a72-c3af-4561-8523-3cd9ebe572fd','9e0ad907-cdf7-484d-96d1-08071a79da19',NULL,NULL,NULL,NULL,NULL);
