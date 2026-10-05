module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [f(2):0] pk = 8'h5a;
  logic [0:f(2)] pa = 8'h5a;
  logic [f(2):-2] pn = 10'h15a;
  initial begin
    $dumpfile("g05g_vcd_u.vcd");
    $dumpvars(0, top);
    #1 pk = 8'ha5; pa = 8'ha5; pn = 10'h2a5;
    #1 $display("done"); $finish;
  end
endmodule
