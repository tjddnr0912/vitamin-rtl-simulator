package q;
  function automatic int h(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
endpackage
module top;
  localparam int P = q::h(2);
  initial begin $display("P=%0d", P); #1 $finish; end
endmodule
