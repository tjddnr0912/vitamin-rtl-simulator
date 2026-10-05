module top;
  typedef enum logic [1:0] {IDLE=2'd0, RUN=2'd1, DONE=2'd2} st_t;
  logic clk, rst_n, go;
  st_t state, nxt;
  logic busy;
  initial begin clk = 1'b0; rst_n = 1'b0; go = 1'b0; #12 rst_n = 1'b1; #10 go = 1'b1; end
  always #5 clk = ~clk;
  always_ff @(posedge clk or negedge rst_n)
    if (!rst_n) state <= IDLE; else state <= nxt;
  always_comb begin
    nxt = state; busy = 1'b0;
    unique case (state)
      IDLE: if (go) nxt = RUN;
      RUN:  begin busy = 1'b1; nxt = DONE; end
      DONE: nxt = IDLE;
    endcase
  end
  always @(posedge clk) $display("t=%0t state=%0d nxt=%0d busy=%b", $time, state, nxt, busy);
  initial #60 $finish;
endmodule
