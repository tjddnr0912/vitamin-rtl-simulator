package q;
  function automatic int f(input int a); return 3; endfunction
  function logic [31:0] h(input int x); return {f(2){1'b1}}; endfunction
endpackage
module top;
  localparam logic [31:0] P = q::h(0);
  initial begin #1 $display("P=%h", P); $finish; end
endmodule
