module t;
  localparam int T [3][2] = '{'{0, 5}, '{7, 0}, '{2, 4}};
  logic [5:0] o;
  for (genvar x = 0; x < 3; x++) begin : gx
    for (genvar y = 0; y < 2; y++) begin : gy
      localparam int Off = T[x][y] % 4;
      if (Off == 0) begin : z
        assign o[x*2+y] = 1'b0;
      end else begin : nz
        assign o[x*2+y] = 1'b1;
      end
    end
  end
  initial begin #1 $display("A o=%b", o); $finish; end
endmodule
