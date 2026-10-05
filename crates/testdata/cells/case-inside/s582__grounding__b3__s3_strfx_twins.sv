module top;
  int m; string t;
  function automatic string sf(input int n); $display("sf(%0d)", n); return (n == 1) ? "b" : "z"; endfunction
  initial begin
    if (sf(1) inside {"a", "c", "b"}) m = 3; else m = 0;   $display("if-inside m=%0d", m);
    m = (sf(1) == "b");                                     $display("eq m=%0d", m);
    t = sf(1); case (t) "a": m = 1; "b": m = 3; default: m = 0; endcase $display("case(var) m=%0d", m);
    case (sf(1)) "b": m = 3; default: m = 0; endcase       $display("case(call) m=%0d", m);
    #10 $finish;
  end
endmodule
