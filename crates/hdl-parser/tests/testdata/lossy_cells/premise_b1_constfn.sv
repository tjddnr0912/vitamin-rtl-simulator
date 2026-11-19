module t;
  function automatic logic [69:0] fx(); return 'x; endfunction
  function automatic logic [7:0] gz(); logic [7:0] r; r = 'z; r[0] = 1'b1; return r; endfunction
  localparam logic [69:0] P = fx();
  localparam logic [7:0] Q = gz();
  initial begin #1 $display("A p=%h q=%b", P, Q); end
  initial #100 $finish;
endmodule
