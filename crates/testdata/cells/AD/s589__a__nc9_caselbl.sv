package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  int x = 2;
  initial begin
    case (x) q::h(18): $display("hit"); default: $display("miss"); endcase
    #1 $finish;
  end
  initial #50 $finish;
endmodule
