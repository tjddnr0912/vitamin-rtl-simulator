module t;
  function automatic logic [69:0] fx(); return 'x; endfunction
  function automatic logic [69:0] fz(); fz = 'z; endfunction
  function automatic logic [69:0] fd(input logic [69:0] a = 'z); return a; endfunction
  logic [69:0] v1, v2, v3;
  initial begin v1 = fx(); v2 = fz(); v3 = fd(); #1 $display("A v1=%h v2=%h v3=%h", v1, v2, v3); end
  initial #100 $finish;
endmodule
