module t;
  logic [69:0] r1, r2, r3, r4;
  function automatic logic [69:0] f9(input int n); logic [69:0] v = 'x; for (int i = 0; i < n; i++) v = 'z; return v; endfunction
  function automatic logic [69:0] fr(input int n); if (n == 0) fr = 'x; else fr = fr(n - 1); endfunction
  task automatic tw(output logic [69:0] o); o = 'z; #1 o = 'x; endtask
  function automatic logic [69:0] fw(input int n); logic [69:0] v; v = 0; while (n > 0) begin v = 'z; n--; end return v; endfunction
  initial begin r1 = f9(0); r2 = fr(3); tw(r3); r4 = fw(2); #1 $display("A r1=%h r2=%h r3=%h r4=%h", r1, r2, r3, r4); r1 = f9(2); #1 $display("B r1=%h", r1); end
  initial #100 $finish;
endmodule
