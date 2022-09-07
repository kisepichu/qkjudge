if [ $# -ne 1 ]; then
	echo usage: source ./migrate.sh {new_version}
	return;
fi

res=`echo 'USE qkjudge; SELECT version FROM migrations ORDER BY id DESC LIMIT 1;' | mysql -uroot`
a=(`echo $res`)
before=${a[1]}
echo current version: $before

after=$1
if [ ${after:1:1} = "0" ]; then
	echo target version $after too old
elif [ $before = $after ]; then
	echo no change
elif [ -f $after.sql ]; then
	diff="out/${before}_to_$after.sql"
	mysqldef -uroot qkjudge < $after.sql > $diff
	query="INSERT INTO migrations (version) VALUES ('$after');"
	res=`echo "USE qkjudge; $query" | mysql -uroot`
	echo $query >> $diff

	res=`echo 'USE qkjudge; SELECT version FROM migrations ORDER BY id DESC LIMIT 1;' | mysql -uroot`
	a=(`echo $res`)
	done=${a[1]}
	echo current version: $done
else
	echo file $after.sql not found
fi
