module dffs #(parameter W = 2) (input logic clk, input logic rst_l, input logic en, input logic [W-1:0] din, output logic [W-1:0] dout);
  always_ff @(posedge clk or negedge rst_l) if (!rst_l) dout <= '0; else if (en) dout <= din;
endmodule
module top;
  logic clk = 0, rst_l = 1;
  logic [1:0][1:0] st, nxt;    // per-entry state, one element per generate iteration
  logic [1:0]      en;
  logic            go = 0;
  for (genvar i = 0; i < 2; i++) begin : genblock
    always_comb begin
      nxt[i] = 2'd0;
      en[i]  = 1'b0;
      case (st[i])
        2'd0: begin nxt[i] = 2'd1; en[i] = go; end
        2'd1: begin nxt[i] = 2'd2; en[i] = 1'b1; end
        default: begin nxt[i] = st[i]; en[i] = 1'b0; end
      endcase
    end
    dffs #(2) ff (.clk(clk), .rst_l(rst_l), .en(en[i]), .din(nxt[i]), .dout({st[i]}));   // concatenation as the output actual
  end
  initial begin
    #1 rst_l = 0; #1 rst_l = 1;
    #1 go = 1;
    repeat (4) begin #5 clk = 1; #5 clk = 0; end
    $display("st=%h", st);
    $finish;
  end
endmodule
