package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    case (x)
      2: h = 4'd3;
      default: h = 4'd1;
    endcase
  endfunction
endpackage
module top;
  logic [3:0] v;
  initial begin v = q::h(2); $display("v=%0d", v); #1 $finish; end
  initial #50 $finish;
endmodule
