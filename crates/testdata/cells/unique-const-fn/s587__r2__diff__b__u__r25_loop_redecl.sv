module top;
  function automatic logic [3:0] fr(input int a);
    fr = 4'd0;
    for (int i = 0; i < 2; i++) begin
      logic [3:0] t;
      if (i == 0) t = 4'd1;
      fr = fr + t;
    end
    unique if (a == 1) fr = 4'd9;
  endfunction
  localparam logic [3:0] PR = fr(2);
  initial begin #1 $display("PR=%b", PR); $finish; end
endmodule
