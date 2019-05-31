(function() {var implementors = {};
implementors["futures"] = [];
implementors["tokio_io"] = [{text:"impl&lt;A&gt; <a class=\"trait\" href=\"futures/stream/trait.Stream.html\" title=\"trait futures::stream::Stream\">Stream</a> for <a class=\"struct\" href=\"tokio_io/io/struct.Lines.html\" title=\"struct tokio_io::io::Lines\">Lines</a>&lt;A&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;A: <a class=\"trait\" href=\"tokio_io/trait.AsyncRead.html\" title=\"trait tokio_io::AsyncRead\">AsyncRead</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/io/trait.BufRead.html\" title=\"trait std::io::BufRead\">BufRead</a>,&nbsp;</span>",synthetic:false,types:["tokio_io::lines::Lines"]},];
implementors["tokio_sync"] = [{text:"impl&lt;T&gt; <a class=\"trait\" href=\"futures/stream/trait.Stream.html\" title=\"trait futures::stream::Stream\">Stream</a> for <a class=\"struct\" href=\"tokio_sync/mpsc/struct.Receiver.html\" title=\"struct tokio_sync::mpsc::Receiver\">Receiver</a>&lt;T&gt;",synthetic:false,types:["tokio_sync::mpsc::bounded::Receiver"]},{text:"impl&lt;T&gt; <a class=\"trait\" href=\"futures/stream/trait.Stream.html\" title=\"trait futures::stream::Stream\">Stream</a> for <a class=\"struct\" href=\"tokio_sync/mpsc/struct.UnboundedReceiver.html\" title=\"struct tokio_sync::mpsc::UnboundedReceiver\">UnboundedReceiver</a>&lt;T&gt;",synthetic:false,types:["tokio_sync::mpsc::unbounded::UnboundedReceiver"]},{text:"impl&lt;T:&nbsp;<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"futures/stream/trait.Stream.html\" title=\"trait futures::stream::Stream\">Stream</a> for <a class=\"struct\" href=\"tokio_sync/watch/struct.Receiver.html\" title=\"struct tokio_sync::watch::Receiver\">Receiver</a>&lt;T&gt;",synthetic:false,types:["tokio_sync::watch::Receiver"]},];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()
