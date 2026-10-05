package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  if (q::h(18) == 4'd2) begin : g
    initial #1 $display("br=then");
  end else begin : e
    initial #1 $display("br=else");
  end
  initial #2 $finish;
  initial #50 $finish;
endmodule
