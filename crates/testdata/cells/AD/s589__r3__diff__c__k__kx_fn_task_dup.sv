package p;
  localparam int K = 8;
  function automatic logic [K-1:0] h(); return '1; endfunction
  task automatic h(output int o); o = 1; endtask
endpackage
module top;
  logic [31:0] v;
  initial begin #1 v = p::h(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
