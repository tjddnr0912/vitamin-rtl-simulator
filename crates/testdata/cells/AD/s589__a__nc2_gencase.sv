package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  case (q::h(18))
    4'd2: begin : a initial #1 $display("arm=2"); end
    5'd18: begin : b initial #1 $display("arm=18"); end
    default: begin : d initial #1 $display("arm=def"); end
  endcase
  initial #2 $finish;
  initial #50 $finish;
endmodule
